#![no_main]
#![no_std]

use core::arch::asm;
use core::mem::size_of;
use core::num::ParseIntError;
use core::ptr::write_bytes;
use core::slice;
use core::str;
use elf_rs::{Elf, ElfFile, ProgramType};
use uefi::table::boot::AllocateType;
use uefi::table::boot::MemoryDescriptor;
use uefi::table::boot::MemoryMap;
use uefi::{
    entry,
    proto::{
        console::{
            gop::GraphicsOutput,
            text::{Color, OutputMode},
        },
        loaded_image::LoadedImage,
        media::{
            file::{File, FileMode},
            fs::SimpleFileSystem,
        },
    },
    table::{boot::MemoryType, Boot, SystemTable},
    CStr16, Handle, Status,
};
use uefi_raw::protocol::file_system::FileAttribute;
use uefi_services::println;
use x86_64::registers::control::EferFlags;

fn bytes_to_str(bytes: &[u8]) -> &str {
    unsafe { str::from_utf8_unchecked(bytes) }
}

fn set_output_mode(system_table: &mut SystemTable<Boot>) {
    let mut largest_mode: Option<OutputMode> = None;
    let mut largest_size = 0;

    for mode in system_table.stdout().modes() {
        if mode.rows() + mode.columns() > largest_size {
            largest_size = mode.rows() + mode.columns();
            largest_mode = Some(mode);
        }
    }

    if let Some(mode) = largest_mode {
        system_table
            .stdout()
            .set_mode(mode)
            .expect("Could not change text mode");
    }
}

fn get_number(data: &[u8]) -> Result<u64, ParseIntError> {
    let mut index = 0;
    for (i, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            index = i;
            break;
        }
    }

    let num_string = bytes_to_str(&data[..index]);

    num_string.trim().parse()
}

#[repr(C)]
#[derive(Debug)]
struct BootInformation<'a> {
    largest_addr: u64,
    framebuffer: *mut u32,
    runtime_system_table: u64,
    memory_map: *mut MemoryMap<'static>,
    kernel_start: u64,
    kernel_end: u64,
    elf_section: Elf<'a>,
    stack_top: u64,
    stack_bottom: u64,
    boot_info_start: u64,
    boot_info_end: u64,
}

const STACK_SIZE: u64 = 4096 * 32;

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    set_output_mode(&mut system_table);
    system_table
        .stdout()
        .set_color(Color::LightGreen, Color::Black)
        .expect("Failed to set Screen color");
    system_table
        .stdout()
        .clear()
        .expect("Could not clear screen");
    let entrypoint: u64;
    let protocol = match system_table
        .boot_services()
        .open_protocol_exclusive::<LoadedImage>(system_table.boot_services().image_handle())
    {
        Ok(protocol) => protocol,
        Err(error) => panic!("could not find protocol for loaded image: {}", error),
    };
    let loaded_image_protocol = match protocol.get() {
        Some(protocol) => protocol,
        None => panic!("Could not get protocol from scoped protocol (Loaded image)"),
    };
    let device_handle = match loaded_image_protocol.device() {
        Some(handle) => handle,
        None => panic!("Could not find device in loaded image protocol"),
    };

    let scoped_simple_file_system = match system_table
        .boot_services()
        .open_protocol_exclusive::<SimpleFileSystem>(device_handle)
    {
        Ok(protocol) => protocol,
        Err(error) => panic!("Could not find protocol for simple file system {:?}", error),
    };
    let simple_file_system = match scoped_simple_file_system.get_mut() {
        Some(protocol) => protocol,
        None => panic!("Could not get protocol from scoped protocol (Simple File System)"),
    };

    let mut root_directory = match simple_file_system.open_volume() {
        Ok(dir) => dir,
        Err(error) => panic!(
            "Could not open volume from simple file system protocol {}",
            error
        ),
    };

    let mut buf = [0; 32];
    let filename = match CStr16::from_str_with_buf("\\boot\\filesize.inf", &mut buf) {
        Ok(filename) => filename,
        Err(error) => panic!("could not create a file name for kernel info {}", error),
    };

    let mut info_file =
        match root_directory.open(&filename, FileMode::Read, FileAttribute::READ_ONLY) {
            Ok(file) => match file.into_regular_file() {
                Some(file) => file,
                None => panic!("A info file for an kernel is not a file"),
            },
            Err(error) => panic!("Could not open info file for the kernel {}", error),
        };

    let mut buffer = [0u8; 64];
    info_file
        .read(&mut buffer)
        .expect("Failed to read kernel info");

    let filesize = match get_number(&buffer) {
        Ok(filesize) => filesize,
        Err(error) => panic!(
            "Could not get a kernel file size from a info data {}",
            error
        ),
    };
    let mut buf = [0; 32];
    let filename = match CStr16::from_str_with_buf("\\boot\\kernel.bin", &mut buf) {
        Ok(filename) => filename,
        Err(error) => panic!("Could not create file name for a kernel {}", error),
    };

    let mut kernel_file =
        match root_directory.open(&filename, FileMode::Read, FileAttribute::READ_ONLY) {
            Ok(file) => match file.into_regular_file() {
                Some(file) => file,
                None => panic!("A kernel file is not a file"),
            },
            Err(error) => panic!("Could not open kernel file {}", error),
        };

    let buffer_ptr = match system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, filesize as usize)
    {
        Ok(ptr) => ptr,
        Err(error) => panic!("Could not allocate buffer for an kernel {}", error),
    };
    let buffer: &mut [u8] = unsafe { slice::from_raw_parts_mut(buffer_ptr, filesize as usize) };
    kernel_file.read(buffer).expect("Failed to load kernel");
    let elf = Elf::from_bytes(buffer).unwrap_or_else(|_| panic!("could not create an elf file"));
    let mut max_alignment: u64 = 4096;
    let mut mem_min: u64 = u64::MAX;
    let mut mem_max: u64 = 0;

    for header in elf.program_header_iter() {
        if header.ph_type() != ProgramType::LOAD {
            continue;
        }

        if max_alignment < header.align() {
            max_alignment = header.align();
        }

        let mut hdr_begin = header.vaddr();
        let mut hdr_end = header.vaddr() + header.memsz() + max_alignment - 1;

        hdr_begin &= !(max_alignment - 1);
        hdr_end &= !(max_alignment - 1);

        if hdr_begin < mem_min {
            mem_min = hdr_begin;
        }
        if hdr_end > mem_max {
            mem_max = hdr_end;
        }
    }

    let max_memory_needed = mem_max - mem_min;
    let count: usize = {
        let padding = mem_min & 0x0fff;
        let total_bytes = max_memory_needed + padding;
        (1 + (total_bytes >> 12)) as usize
    };

    let stack_ptr = match system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, STACK_SIZE as usize)
    {
        Ok(ptr) => ptr,
        Err(err) => {
            panic!("Failed to allocate memory for the kernel stack {:?}", err);
        }
    };

    let program_ptr = match system_table.boot_services().allocate_pages(
        AllocateType::Address(mem_min),
        MemoryType::LOADER_DATA,
        count,
    ) {
        Ok(ptr) => ptr as *mut u8,
        Err(err) => {
            panic!("Failed to allocate memory for the kernel {:?}", err);
        }
    };

    unsafe {
        write_bytes(program_ptr, 0, max_memory_needed as usize);
    }

    for header in elf.program_header_iter() {
        if header.ph_type() != ProgramType::LOAD {
            continue;
        }

        let relative_offset = header.vaddr() - mem_min;

        let dst = program_ptr as u64 + relative_offset;
        let src = buffer_ptr as u64 + header.offset();
        let len = header.filesz();

        unsafe {
            core::ptr::copy(src as *const u8, dst as *mut u8, len as usize);
        }
    }
    entrypoint = program_ptr as u64 + (elf.elf_header().entry_point() - mem_min);

    let boot_info = system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_CODE, size_of::<BootInformation>())
        .unwrap_or_else(|e| panic!("Failed to allocate memory for the boot information {}", e))
        as *mut BootInformation;
    unsafe {
        (&mut *boot_info).kernel_start = mem_min;
        (&mut *boot_info).kernel_end = mem_max;
        (&mut *boot_info).elf_section = elf;
        (&mut *boot_info).stack_top = stack_ptr as u64 + STACK_SIZE;
        (&mut *boot_info).stack_bottom = stack_ptr as u64;
        (&mut *boot_info).boot_info_start = boot_info as u64;
        (&mut *boot_info).boot_info_end = boot_info as u64 + size_of::<BootInformation>() as u64;
    }

    drop(protocol);
    drop(scoped_simple_file_system);
    println!("Press any key to boot...");

    loop {
        match system_table.stdin().read_key() {
            Ok(key) => match key {
                Some(_) => break,
                None => {}
            },
            Err(err) => {
                panic!("Failed to read key: {}", err);
            }
        }
    }

    let handle = system_table
        .boot_services()
        .get_handle_for_protocol::<GraphicsOutput>();
    let gop = system_table
        .boot_services()
        .open_protocol_exclusive::<GraphicsOutput>(handle.unwrap());
    let mut gop = gop.unwrap();
    let framebuffer = gop.frame_buffer().as_mut_ptr() as usize;
    unsafe {
        (&mut *boot_info).framebuffer = framebuffer as *mut u32;
    }
    for mode in gop.modes(system_table.boot_services()) {
        if mode.info().resolution() == (1920, 1080) {
            gop.set_mode(&mode).expect("Could not set mode");
            break;
        }
    }
    drop(gop);
    let (system_table, mut memory_map) = system_table.exit_boot_services(MemoryType::LOADER_CODE);
    unsafe {
        (&mut *boot_info).memory_map = &mut memory_map as *mut MemoryMap<'static>;
        (&mut *boot_info).runtime_system_table = system_table.get_current_system_table_addr();
        (&mut *boot_info).largest_addr = [
            &memory_map as *const MemoryMap<'static> as u64 + size_of::<MemoryMap>() as u64 - 1,
            memory_map.entries().last().unwrap() as *const MemoryDescriptor as u64
                + size_of::<MemoryDescriptor>() as u64
                - 1,
            boot_info as u64 + size_of::<BootInformation>() as u64 - 1,
            buffer_ptr.add(filesize as usize) as u64 - 1,
        ]
        .iter()
        .max()
        .unwrap()
            / 0x40000000
            + 1;
    }
    unsafe {
        x86_64::registers::model_specific::Efer::write(EferFlags::LONG_MODE_ENABLE);
    }

    unsafe {
        asm!(
            r#"
            xor rbp, rbp
            mov rsp, {}
            jmp {}
        "#,
        in(reg) stack_ptr.add(STACK_SIZE as usize),
        in(reg) entrypoint,
        in("rdi") boot_info
        );
    }

    unreachable!();
}
