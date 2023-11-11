#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(nothingos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate spin;
extern crate alloc;
extern crate core;
extern crate nothingos;
extern crate multiboot2;
extern crate lazy_static;

use lazy_static::lazy_static;
use multiboot2::{BootInformation, BootInformationHeader};
use nothingos::print::Print;
use nothingos::println;
use spin::Mutex;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init();
    let bootinfo = unsafe { BootInformation::load(multiboot_information_address).unwrap() };
    let memory_map_tag = bootinfo.memory_map_tag()
        .expect("Memory map tag required");
    println!("{}", memory_map_tag.memory_areas().get(1).unwrap().size());
    #[cfg(test)]
    test_main();
    hlt_loop(); 
}
