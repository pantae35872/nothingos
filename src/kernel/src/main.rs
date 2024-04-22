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

use nothingos::task::executor::{AwaitType, Executor};
use nothingos::{driver, serial_println, BootInformation};
use uefi::proto::console::gop::PixelFormat;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn start(information_address: *mut BootInformation) -> ! {
    let boot_info = unsafe { &mut *information_address };
    nothingos::init(information_address);
    let mut executor = Executor::new();
    executor.spawn(
        async {
            /*let mut controller = ahci_driver::DRIVER
                .get()
                .expect("AHCI Driver is not initialize")
                .lock();
            let drive = controller.get_drive(&0).await.expect("Cannot get drive");
            let mut data: [u8; 8196] = [0u8; 8196];
            drive.identify().await.expect("could not identify drive");
            drive.read(0, &mut data, 1).await.expect("Read failed");

            let mut gpt = GPTPartitions::new(drive).await.expect("Error");
            let partition1 = gpt.read_partition(2).await.expect("");

            println!("{}", partition1.get_partition_name());*/
        },
        AwaitType::AlwaysPoll,
    );
    executor.spawn(
        async {
            if boot_info.gop_mode.info().pixel_format() == PixelFormat::Rgb {
                serial_println!("This is rgb");
                let (width, height) = boot_info.gop_mode.info().resolution();
                for y in 0..height {
                    for x in 0..width {
                        unsafe {
                            (*boot_info.framebuffer.wrapping_add(y * width + x)) = 0x00FFFFFF;
                        }
                    }
                }
            } else if boot_info.gop_mode.info().pixel_format() == PixelFormat::Bgr {
                serial_println!("This is bgr");
                let (width, height) = boot_info.gop_mode.info().resolution();
                for y in 0..height {
                    for x in 0..width {
                        unsafe {
                            (*boot_info.framebuffer.wrapping_add(y * width + x)) = 0x0000FFFF;
                        }
                    }
                }
            }
        },
        AwaitType::AlwaysPoll,
    );

    executor.spawn(driver::timer::timer_task(), AwaitType::WakePoll);
    executor.spawn(driver::keyboard::keyboard_task(), AwaitType::WakePoll);

    #[cfg(test)]
    test_main();

    executor.run();
}
