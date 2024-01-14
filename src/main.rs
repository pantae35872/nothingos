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

use alloc::vec;
use alloc::vec::Vec;
use multiboot2::BootInformationHeader;
use nothingos::driver::storage::ahci_driver::AHCIDrive;
use nothingos::driver::storage::ata_driver::ATADrive;
use nothingos::filesystem::partition::gpt_partition::GPTPartitions;
use nothingos::task::executor::{AwaitType, Executor};
use nothingos::{driver, println};
use uguid::guid;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub fn start(multiboot_information_address: *const BootInformationHeader) -> ! {
    nothingos::init(multiboot_information_address);
    let mut executor = Executor::new();
    executor.spawn(
        async {
            AHCIDrive::new().await;
            /*let mut drive = ATADrive::new(0x1F0, true);
            drive.identify().await;

            let mut gpt = GPTPartitions::new(&mut drive).await.expect("Error");
            gpt.format().await.expect("format partition error");
            gpt.set_partiton(
                1,
                &guid!("0FC63DAF-8483-4772-8E79-3D69D8477DE4"),
                34,
                1058,
                0,
                &{
                    let mut array = [0; 72];
                    let string: Vec<u8> = "Linux filesystem"
                        .encode_utf16()
                        .flat_map(|c| vec![(c & 0xFF) as u8, (c >> 8) as u8])
                        .collect();
                    array[..string.len()].copy_from_slice(string.as_slice());
                    array
                },
            )
            .await
            .expect("Write partition error");
            gpt.set_partiton(
                2,
                &guid!("0FC63DAF-8483-4772-8E79-3D69D8477DE4"),
                1059,
                2082,
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
            .await
            .expect("Write partition error");
            let abc = gpt.read_partition(2).await.expect("");
            println!("{:?}", abc.get_partition_name());*/
        },
        AwaitType::AlwaysPoll,
    );

    executor.spawn(driver::timer::timer_task(), AwaitType::WakePoll);
    executor.spawn(driver::keyboard::keyboard_task(), AwaitType::WakePoll);

    #[cfg(test)]
    test_main();

    executor.run();
}
