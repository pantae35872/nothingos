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

use multiboot2::BootInformationHeader;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init(multiboot_information_address);
    #[cfg(test)]
    test_main();
    hlt_loop(); 
}


