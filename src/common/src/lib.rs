#![no_std]

extern crate alloc;
extern crate core;

pub mod hash_map;
pub mod toml;

use elf_rs::Elf;
use uefi::{proto::console::gop::Mode, table::boot::MemoryMap};

#[repr(C)]
#[derive(Debug)]
pub struct BootInformation {
    pub largest_addr: u64,
    pub gop_mode: Mode,
    pub framebuffer: *mut u32,
    pub runtime_system_table: u64,
    pub memory_map: *mut MemoryMap<'static>,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub elf_section: Elf<'static>,
    pub boot_info_start: u64,
    pub boot_info_end: u64,
    pub font_start: u64,
    pub font_end: u64,
}
