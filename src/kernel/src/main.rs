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

use core::arch::asm;

use alloc::ffi::CString;
use alloc::vec;
use alloc::vec::Vec;
use nothingos::driver::storage::{ahci_driver, Drive};
use nothingos::filesystem::partition::gpt_partition::GPTPartitions;
use nothingos::{hlt_loop, println, BootInformation};
use uguid::guid;

#[no_mangle]
fn sys_print(value: &str) {
    let string = CString::new(value).unwrap();
    unsafe {
        asm!("int 0x80", in("rax") 1, in("rcx") string.into_raw());
    }
}

#[no_mangle]
pub extern "C" fn start(information_address: *mut BootInformation) -> ! {
    nothingos::init(information_address);
    println!("Hello worldaaa!!!!");
    let mut controller = ahci_driver::DRIVER
        .get()
        .expect("AHCI Driver is not initialize")
        .lock();
    let mut abc = [0u8; 8192];
    let drive = controller.get_drive(&0).expect("Cannot get drive");
    drive.read(0, &mut abc, 16).unwrap();
    println!("{:?}", abc);
    let mut gpt = GPTPartitions::new(drive).expect("Error");
    gpt.format().unwrap();
    gpt.set_partiton(
        1,
        &guid!("0FC63DAF-8483-4772-8E79-3D69D8477DE4"),
        34,
        2048,
        0,
        &{
            let mut array = [0; 72];
            let string: Vec<u8> = "My partition"
                .encode_utf16()
                .flat_map(|c| vec![(c & 0xFF) as u8, (c >> 8) as u8])
                .collect();
            array[..string.len()].copy_from_slice(string.as_slice());
            array
        },
    )
    .unwrap();
    let partition1 = gpt.read_partition(1).expect("Error");
    println!("{}", partition1.get_partition_name());

    #[cfg(test)]
    test_main();

    hlt_loop();
}
