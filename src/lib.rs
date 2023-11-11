#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

extern crate spin;
extern crate alloc;
extern crate core;
extern crate multiboot2;
extern crate lazy_static;

pub mod serial;
pub mod print;
pub mod gdt;
pub mod memory;
pub mod interrupt;
pub mod utils;
pub mod allocator;

use core::panic::PanicInfo;

use multiboot2::BootInformation;
use x86_64::VirtAddr;

use self::interrupt::PICS;
//use self::memory::{BootInfoFrameAllocator, init_heap};
use self::print::PRINT;
pub trait Testable {
    fn run(&self) -> ();
}


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

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

pub fn init() {
    PRINT.lock().set_color(&0xb, &0);
    gdt::init();
    interrupt::init_idt();
    unsafe { PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    /*let elf_sections_tag = boot_info.elf_sections()
    .expect("Elf-sections tag required");
    let kernel_start = elf_sections_tag.map(|s| s.start_address())
        .min().unwrap();
    let kernel_end = elf_sections_tag.map(|s| s.end_address())
        .max().unwrap();
    let phys_mem_offset = VirtAddr::new(kernel_start);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");*/
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
