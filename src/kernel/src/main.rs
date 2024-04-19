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

use core::fmt::Write;

use nothingos::driver::storage::{ahci_driver, Drive};
use nothingos::filesystem::partition::gpt_partition::GPTPartitions;
use nothingos::task::executor::{AwaitType, Executor};
use nothingos::{driver, println, serial_println, BootInformation};
use uart_16550::SerialPort;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn start(information_address: *mut BootInformation) -> ! {
    nothingos::init(information_address);
    serial_println!("Hello world");
    /*let boot_info = unsafe { &mut *information_address };
    for y in 0..1080 {
        for x in 0..1920 {
            unsafe {
                (*boot_info.framebuffer.wrapping_add(y * 1920 + x)) = 0xFFFFFFFF;
            }
        }
    }*/
    let mut executor = Executor::new();
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();
    serial_port.write_str("Hello world\n").expect("aaa");
    executor.spawn(
        async {
            let mut controller = ahci_driver::DRIVER
                .get()
                .expect("AHCI Driver is not initialize")
                .lock();
            let drive = controller.get_drive(&0).await.expect("Cannot get drive");
            let mut data: [u8; 8196] = [0u8; 8196];
            drive.identify().await.expect("could not identify drive");
            drive.read(0, &mut data, 1).await.expect("Read failed");

            let mut gpt = GPTPartitions::new(drive).await.expect("Error");
            let partition1 = gpt.read_partition(2).await.expect("");

            println!("{}", partition1.get_partition_name());
        },
        AwaitType::AlwaysPoll,
    );

    executor.spawn(driver::timer::timer_task(), AwaitType::WakePoll);
    executor.spawn(driver::keyboard::keyboard_task(), AwaitType::WakePoll);

    #[cfg(test)]
    test_main();

    executor.run();
}
