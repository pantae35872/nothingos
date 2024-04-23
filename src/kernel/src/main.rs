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
use nothingos::graphics::{draw_line, draw_triangle};
use nothingos::print::ttf_parser::TtfParser;
use nothingos::task::executor::{AwaitType, Executor};
use nothingos::utils::math::{Coordinate, Polygon};
use nothingos::{driver, serial_print, serial_println, BootInformation};
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
                /*let triangle = draw_triangle(
                    &Coordinate::new(100, 100),
                    &Coordinate::new(200, 200),
                    &Coordinate::new(200, 100),
                );
                for line in triangle {
                    for point in line {
                        unsafe {
                            (*boot_info.framebuffer.wrapping_add(
                                point.get_x() as usize * width + point.get_y() as usize,
                            )) = 0x0000FFFF;
                        }
                    }
                }*/
            }
            let (width, height) = boot_info.gop_mode.info().resolution();
            let font = unsafe {
                core::slice::from_raw_parts_mut(
                    boot_info.font_start as *mut u8,
                    (boot_info.font_end - boot_info.font_start) as usize,
                )
            };
            let mut font_parser = TtfParser::new(font);
            let polygons = font_parser.test();
            let mut offset = 1;
            let mut y_offset = 1;
            for mut polygon in polygons {
                polygon.flip();
                polygon.scale(0.1);
                polygon.move_down(y_offset * 100);
                for line in polygon.vertices() {
                    for point in line {
                        unsafe {
                            (*boot_info.framebuffer.wrapping_add(
                                point.get_y() as usize * width
                                    + point.get_x() as usize
                                    + (offset * 100),
                            )) = 0x0000FFFF;
                        }
                    }
                }
                offset += 1;
                if offset > 8 {
                    y_offset += 1;
                    offset = 1;
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
