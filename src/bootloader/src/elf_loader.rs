use core::ptr::write_bytes;

use common::boot::BootInformation;
use elf_rs::{Elf, ElfFile, ProgramType};
use uefi::table::{
    boot::{AllocateType, MemoryType},
    Boot, SystemTable,
};
use uefi_services::println;

pub fn load_elf(
    system_table: &mut SystemTable<Boot>,
    buffer: &'static [u8],
    boot_info: &mut BootInformation,
) -> u64 {
    let elf = Elf::from_bytes(buffer).unwrap_or_else(|_| panic!("could not create an elf file"));
    elf.program_header_iter().for_each(|e| println!("{:?}", e));
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
        let src = buffer.as_ptr() as u64 + header.offset();
        let len = header.filesz();

        unsafe {
            core::ptr::copy(src as *const u8, dst as *mut u8, len as usize);
        }
    }

    boot_info.kernel_start = mem_min;
    boot_info.kernel_end = mem_max;
    boot_info.elf_section = elf;
    return program_ptr as u64 + (boot_info.elf_section.elf_header().entry_point() - mem_min);
}
