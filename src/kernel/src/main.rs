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
extern crate nothingos;
extern crate spin;

use core::f64::consts::PI;

use alloc::vec::Vec;
use nothingos::driver::storage::{ahci_driver, Drive};
use nothingos::filesystem::partition::gpt_partition::GPTPartitions;
use nothingos::graphics::apply_alpha;
use nothingos::print::ttf_parser::TtfParser;
use nothingos::print::ttf_renderer::{self, TtfRenderer};
use nothingos::task::executor::{AwaitType, Executor};
use nothingos::{driver, graphics, println, BootInformation};
use uefi::proto::console::gop::PixelFormat;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn start(information_address: *mut BootInformation) -> ! {
    nothingos::init(information_address);
    println!("Hello world!");
    let mut executor = Executor::new();
    executor.spawn(
        async {
            let mut controller = ahci_driver::DRIVER
                .get()
                .expect("AHCI Driver is not initialize")
                .lock();
            let drive = controller.get_drive(&0).await.expect("Cannot get drive");
            //let mut data: [u8; 8196] = [0u8; 8196];
            println!("Async !!!!");
            drive.identify().await.expect("could not identify drive");
            println!("Async !!!!");
            println!("{}", drive.lba_end());
            /*let mut gpt = GPTPartitions::new(drive).await.expect("Error");
            let partition1 = gpt.read_partition(0).await.expect("Error");

            println!("{}", partition1.get_partition_name());*/
        },
        AwaitType::AlwaysPoll,
    );
    executor.spawn(async {}, AwaitType::AlwaysPoll);

    executor.spawn(driver::timer::timer_task(), AwaitType::WakePoll);
    executor.spawn(driver::keyboard::keyboard_task(), AwaitType::WakePoll);

    #[cfg(test)]
    test_main();

    executor.run();
}
