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
use nothingos::graphics::frame_renderer::FrameRenderer;
use nothingos::graphics::{draw_line, frame_renderer};
use nothingos::print::ttf_parser::TtfParser;
use nothingos::task::executor::{AwaitType, Executor};
use nothingos::utils::math::Polygon;
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
            font_parser.parse().unwrap();
            let mut offset = 1;
            let mut y_offset = 1;
            let mut polygons = Vec::new();
            polygons.push(font_parser.draw_char(&'A'));
            polygons.push(font_parser.draw_char(&'B'));
            polygons.push(font_parser.draw_char(&'C'));
            polygons.push(font_parser.draw_char(&'D'));
            polygons.push(font_parser.draw_char(&'E'));
            polygons.push(font_parser.draw_char(&'F'));
            polygons.push(font_parser.draw_char(&'G'));
            polygons.push(font_parser.draw_char(&'H'));
            polygons.push(font_parser.draw_char(&'I'));
            polygons.push(font_parser.draw_char(&'J'));
            polygons.push(font_parser.draw_char(&'K'));
            polygons.push(font_parser.draw_char(&'L'));
            polygons.push(font_parser.draw_char(&'M'));
            polygons.push(font_parser.draw_char(&'N'));
            polygons.push(font_parser.draw_char(&'O'));
            polygons.push(font_parser.draw_char(&'P'));
            polygons.push(font_parser.draw_char(&'Q'));
            polygons.push(font_parser.draw_char(&'R'));
            polygons.push(font_parser.draw_char(&'S'));
            polygons.push(font_parser.draw_char(&'T'));
            polygons.push(font_parser.draw_char(&'U'));
            polygons.push(font_parser.draw_char(&'V'));
            polygons.push(font_parser.draw_char(&'W'));
            polygons.push(font_parser.draw_char(&'X'));
            polygons.push(font_parser.draw_char(&'Y'));
            polygons.push(font_parser.draw_char(&'Z'));
            polygons.push(font_parser.draw_char(&'a'));
            polygons.push(font_parser.draw_char(&'b'));
            polygons.push(font_parser.draw_char(&'c'));
            polygons.push(font_parser.draw_char(&'d'));
            polygons.push(font_parser.draw_char(&'e'));
            polygons.push(font_parser.draw_char(&'f'));
            polygons.push(font_parser.draw_char(&'g'));
            polygons.push(font_parser.draw_char(&'h'));
            polygons.push(font_parser.draw_char(&'i'));
            polygons.push(font_parser.draw_char(&'j'));
            polygons.push(font_parser.draw_char(&'k'));
            polygons.push(font_parser.draw_char(&'l'));
            polygons.push(font_parser.draw_char(&'m'));
            polygons.push(font_parser.draw_char(&'n'));
            polygons.push(font_parser.draw_char(&'o'));
            polygons.push(font_parser.draw_char(&'p'));
            polygons.push(font_parser.draw_char(&'q'));
            polygons.push(font_parser.draw_char(&'r'));
            polygons.push(font_parser.draw_char(&'s'));
            polygons.push(font_parser.draw_char(&'t'));
            polygons.push(font_parser.draw_char(&'u'));
            polygons.push(font_parser.draw_char(&'v'));
            polygons.push(font_parser.draw_char(&'w'));
            polygons.push(font_parser.draw_char(&'x'));
            polygons.push(font_parser.draw_char(&'y'));
            polygons.push(font_parser.draw_char(&'z'));
            for mut polygon in polygons {
                //polygon.flip();
                polygon.scale(0.1);
                polygon.flip();
                //polygon.fill();
                polygon.move_down(y_offset as f32 * 100.0);
                for pixel in polygon.render() {
                    unsafe {
                        (*boot_info
                            .framebuffer
                            .wrapping_add(pixel.y() * width + pixel.x() + (offset * 100))) =
                            0x00FFFFFF;
                    }
                }
                offset += 1;
                if offset > 15 {
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
