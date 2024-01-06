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

use alloc::string::String;
use multiboot2::BootInformationHeader;
use nothingos::driver::storage::ata_driver::ATADrive;
use nothingos::task::executor::Executor;
use nothingos::{println, driver};

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init(multiboot_information_address);
    let mut executor = Executor::new();
    executor.spawn(async {
        let mut drive = ATADrive::new(0x1F0, true);
        drive.identify().await;
        //let msg = "Hello from write".as_bytes();
        //drive.write28(0, msg, msg.len()).await;
        let mut msg = [0u8; 16];
        drive.read28(0, &mut msg, 16).await;
        println!("{}", String::from_utf8_lossy(&msg));
    });
    
    executor.spawn(driver::timer::timer_task());
    executor.spawn(driver::keyboard::keyboard_task());
    
    #[cfg(test)]
    test_main();
   
    executor.run();
}
