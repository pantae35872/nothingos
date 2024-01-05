#![no_std]
#![feature(pointer_is_aligned)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(const_mut_refs)]
#[macro_use]
extern crate bitflags;

bitflags! {
    #[derive(Clone, Copy)]
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

impl EntryFlags {
    pub fn from_elf_section_flags(section: &ElfSection) -> EntryFlags {
        use multiboot2::ElfSectionFlags;

        let mut flags = EntryFlags::empty();

        if section.flags().contains(ElfSectionFlags::ALLOCATED) {
            // section is loaded to memory
            flags = flags | EntryFlags::PRESENT;
        }
        if section.flags().contains(ElfSectionFlags::WRITABLE) {
            flags = flags | EntryFlags::WRITABLE;
        }
        if !section.flags().contains(ElfSectionFlags::EXECUTABLE) {
            flags = flags | EntryFlags::NO_EXECUTE;
        }

        flags
    }
}

extern crate alloc;
extern crate core;
extern crate lazy_static;
extern crate multiboot2;
extern crate spin;

#[cfg(feature = "test")]
pub mod serial;

pub mod allocator;
pub mod gdt;
pub mod interrupt;
pub mod memory;
pub mod print;
pub mod utils;
pub mod vga;
pub mod gui;
pub mod renderer;
pub mod task;
pub mod driver;

use core::panic::PanicInfo;

use multiboot2::{BootInformation, BootInformationHeader, ElfSection};
use x86_64::registers::model_specific::EferFlags;

use self::allocator::{HEAP_SIZE, HEAP_START, init_heap};
use self::interrupt::PICS;
use self::memory::paging::Page;
use self::print::PRINT;
use self::vga::{BACKBUFFER_SIZE, BACKBUFFER_START};

pub trait Testable {
    fn run(&self) -> ();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    #[cfg(feature = "test")]
    {
        test_panic_handler(info);
    }
    #[cfg(not(feature = "test"))]
    hlt_loop();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        #[cfg(feature = "test")]
        serial_print!("{}...\t", core::any::type_name::<T>());
        #[cfg(feature = "test")]
        self();
        #[cfg(feature = "test")]
        serial_println!("[ok]");
    }
}

pub fn test_runner(_tests: &[&dyn Testable]) {
    #[cfg(feature = "test")]
    serial_println!("Running {} tests", _tests.len());
    #[cfg(feature = "test")]
    for test in _tests {
        test.run();
    }
    #[cfg(feature = "test")]
    exit_qemu(QemuExitCode::Success);
}

#[cfg(feature = "test")]
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(multiboot_information_address: *const BootInformationHeader) {
    PRINT.lock().set_color(&0xb, &0);
    gdt::init();
    interrupt::init_idt();
    unsafe { PICS.lock().initialize() };
    let boot_info = unsafe { BootInformation::load(multiboot_information_address).unwrap() };
    let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");
    let kernel_start = elf_sections_tag
        .clone()
        .map(|s| s.start_address())
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .clone()
        .map(|s| s.end_address())
        .max()
        .unwrap();
    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start as usize + (boot_info.total_size() as usize);
    let mut frame_allocator = memory::area_frame_allocator::AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        multiboot_start as usize,
        multiboot_end,
        boot_info.memory_map_tag().unwrap().memory_areas(),
    );
    enable_nxe_bit();
    let mut active_table = memory::remap_the_kernel(&mut frame_allocator, &boot_info);
    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
    }
    let backbuffer_start_page = Page::containing_address(BACKBUFFER_START);
    let backbuffer_end_page = Page::containing_address(BACKBUFFER_START + BACKBUFFER_SIZE - 1);

    for page in Page::range_inclusive(backbuffer_start_page, backbuffer_end_page) {
        active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
    }
    init_heap();
    driver::init();
    task::init();
}

fn enable_nxe_bit() {
    use x86_64::registers::model_specific::Efer;

    unsafe {
        Efer::write(EferFlags::NO_EXECUTE_ENABLE);
    }
}

#[cfg(feature = "test")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[cfg(feature = "test")]
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
