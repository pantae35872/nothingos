#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(nothingos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate core;
extern crate lazy_static;
extern crate multiboot2;
extern crate nothingos;
extern crate spin;

use core::cell::RefCell;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use multiboot2::BootInformationHeader;
use nothingos::drive::ATADrive;
use nothingos::gui::{RootWidget, Widget, WindowWidget};
use nothingos::port::{Port8Bit, Port16Bit, inb, outb, inw};
use nothingos::print::PRINT;
use nothingos::println;
use nothingos::vga::VGA;
use x86_64::instructions::port;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn stack_overflow() {
    stack_overflow();
}

#[no_mangle]
pub fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init(multiboot_information_address);
    let mut drive = ATADrive::new(0x1F0, true);
    drive.identify();
    
    let strt: Vec<String> = vec![String::from("tinnaa"), String::from("hellop")];
    println!("It not crash {}, {}", strt[0], strt[1]);
    #[cfg(test)]
    test_main();
    hlt_loop();
}
