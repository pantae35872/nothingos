#![no_main]
#![no_std]
#![feature(str_from_raw_parts)]
#![feature(allocator_api)]
extern crate alloc;
use crate::toml::TomlValue;
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::arch::asm;
use core::mem::size_of;
use core::ptr::write_bytes;
use core::slice;
use core::str;
use elf_rs::{Elf, ElfFile, ProgramType};
use uefi::proto::console::gop::Mode;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::RegularFile;
use uefi::table::boot::AllocateType;
use uefi::table::boot::MemoryDescriptor;
use uefi::table::boot::MemoryMap;
use uefi::table::boot::OpenProtocolAttributes;
use uefi::table::boot::OpenProtocolParams;
use uefi::{
    entry,
    proto::{
        console::{gop::GraphicsOutput, text::OutputMode},
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

pub mod toml;

#[repr(C)]
#[derive(Debug)]
struct BootInformation {
    largest_addr: u64,
    gop_mode: Mode,
    framebuffer: *mut u32,
    runtime_system_table: u64,
    memory_map: *mut MemoryMap<'static>,
    kernel_start: u64,
    kernel_end: u64,
    elf_section: Elf<'static>,
    boot_info_start: u64,
    boot_info_end: u64,
    font_start: u64,
    font_end: u64,
}

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    set_output_mode(&mut system_table);
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
    let filename = match CStr16::from_str_with_buf("\\boot\\bootinfo.toml", &mut buf) {
        Ok(filename) => filename,
        Err(error) => panic!("could not create a file name for kernel info {}", error),
    };

    let mut info_file: RegularFile =
        match root_directory.open(&filename, FileMode::Read, FileAttribute::READ_ONLY) {
            Ok(file) => match file.into_regular_file() {
                Some(file) => file,
                None => panic!("A info file for an kernel is not a file"),
            },
            Err(error) => panic!("Could not open info file for the kernel {}", error),
        };

    let mut buffer = [0u8; 512];
    let info: &mut FileInfo = match info_file.get_info(&mut buffer) {
        Ok(value) => value,
        Err(panic) => panic!("Could not get info for a kernel info file: {}", panic),
    };

    let info_buffer: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            system_table
                .boot_services()
                .allocate_pool(MemoryType::LOADER_DATA, info.file_size() as usize)
                .unwrap(),
            info.file_size() as usize,
        )
    };

    info_file
        .read(info_buffer)
        .expect("Could not get file for kernel info");

    let info_file =
        unsafe { core::str::from_raw_parts(info_buffer.as_ptr(), info.file_size() as usize) };

    let info_file: Vec<(&str, TomlValue)> = toml::parse_toml(info_file).unwrap();

    let mut kernel_file: &str = "";
    let mut kernel_font_file: &str = "";
    for (key, value) in info_file {
        if key == "kernel_file" {
            match value {
                TomlValue::String(s) => {
                    kernel_file = s;
                }
                TomlValue::Integer(_) => panic!("Kernel file is not a string"),
            }
        } else if key == "font_file" {
            match value {
                TomlValue::String(s) => {
                    kernel_font_file = s;
                }
                TomlValue::Integer(_) => panic!("Kernel font file is not a string"),
            }
        }
    }
    let mut buf = [0; 64];
    let filename = match CStr16::from_str_with_buf(
        ("\\boot\\".to_owned() + kernel_font_file).as_str(),
        &mut buf,
    ) {
        Ok(filename) => filename,
        Err(error) => panic!("Could not create file name for a kernel {}", error),
    };

    let mut kernel_font_file =
        match root_directory.open(&filename, FileMode::Read, FileAttribute::READ_ONLY) {
            Ok(file) => match file.into_regular_file() {
                Some(file) => file,
                None => panic!("A kernel file is not a file"),
            },
            Err(error) => panic!("Could not open kernel file {}", error),
        };
    let mut kernel_font_info_aaaaaaaaaa = [0u8; 256];
    let kernel_font_filesize = kernel_font_file
        .get_info::<FileInfo>(&mut kernel_font_info_aaaaaaaaaa)
        .unwrap()
        .file_size();

    let font_buffer_ptr = match system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, kernel_font_filesize as usize)
    {
        Ok(ptr) => ptr,
        Err(error) => panic!("Could not allocate buffer for an kernel {}", error),
    };
    let font_buffer: &mut [u8] =
        unsafe { slice::from_raw_parts_mut(font_buffer_ptr, kernel_font_filesize as usize) };

    kernel_font_file
        .read(font_buffer)
        .expect("Failed to load kernel font file");

    let mut buf = [0; 64];
    let filename =
        match CStr16::from_str_with_buf(("\\boot\\".to_owned() + kernel_file).as_str(), &mut buf) {
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
    let mut kernel_info_aaaaaaaaaa = [0u8; 256];
    let filesize = kernel_file
        .get_info::<FileInfo>(&mut kernel_info_aaaaaaaaaa)
        .unwrap()
        .file_size();

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
        (&mut *boot_info).boot_info_start = boot_info as u64;
        (&mut *boot_info).boot_info_end = boot_info as u64 + size_of::<BootInformation>() as u64;
        (&mut *boot_info).font_start = font_buffer_ptr as u64;
        (&mut *boot_info).font_end = font_buffer_ptr as u64 + kernel_font_filesize - 1;
    }

    drop(protocol);
    drop(scoped_simple_file_system);

    let handle = system_table
        .boot_services()
        .get_handle_for_protocol::<GraphicsOutput>();
    let gop = unsafe {
        system_table
            .boot_services()
            .open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: handle.unwrap(),
                    agent: system_table.boot_services().image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
    };
    let mut gop = gop.unwrap();
    let framebuffer = gop.frame_buffer().as_mut_ptr() as usize;
    unsafe {
        (&mut *boot_info).framebuffer = framebuffer as *mut u32;
    }
    for mode in gop.modes(system_table.boot_services()) {
        if mode.info().resolution() == (1920, 1080) {
            gop.set_mode(&mode).expect("Could not set mode");
            unsafe {
                (&mut *boot_info).gop_mode = mode;
            }
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
        asm!(
            r#"
            xor rbp, rbp
            jmp {}
        "#,
        in(reg) entrypoint,
        in("rdi") boot_info
        );
    }

    unreachable!();
}
