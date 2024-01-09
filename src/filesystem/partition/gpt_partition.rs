use core::mem::size_of;

use alloc::boxed::Box;
use alloc::string::String;
use uguid::Guid;

use crate::driver::storage::ata_driver::ATADrive;
use crate::driver::storage::CHS;
use crate::println;
use crate::utils::oserror::OSError;

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct ProtectiveMasterBootRecord {
    bootable: u8,
    start_chs: CHS,
    os_type: u8,
    end_chs: CHS,
    start_lba: u32,
    end_lba: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct PartitionTableHeader {
    signature: u64,
    gpt_revision: u32,
    header_size: u32,
    checksum: u32,
    reserved: u32,
    header_lba: u64,
    alternate_header_lba: u64,
    start_usable: u64,
    end_usable: u64,
    guid: Guid,
    start_partition_entry_lba: u64,
    number_partition_entries: u32,
    partition_entry_size: u32,
    partition_entry_array_checksum: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct PartitionEntry {
    partition_type: Guid,
    guid: Guid,
    start_lba: u64,
    end_lba: u64,
    attributes: u64,
    partition_name: [u8; 72],
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct PartitionEntries {
    entries: [PartitionEntry; 4],
}

impl PartitionTableHeader {
    pub fn new() -> Self {
        Self {
            signature: 0,
            gpt_revision: 0,
            header_size: 0,
            checksum: 0,
            reserved: 0,
            header_lba: 0,
            alternate_header_lba: 0,
            start_usable: 0,
            end_usable: 0,
            guid: Guid::ZERO,
            start_partition_entry_lba: 0,
            number_partition_entries: 0,
            partition_entry_size: 0,
            partition_entry_array_checksum: 0,
        }
    }
}

pub async fn test1(drive: &mut ATADrive) -> Result<(), Box<OSError>> {
    let mut header = PartitionTableHeader::new();
    let header_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            &mut header as *mut _ as *mut u8,
            size_of::<PartitionTableHeader>(),
        )
    };
    drive
        .read28(1, header_bytes, size_of::<PartitionTableHeader>())
        .await?;

    println!(
        "{:?}\n{}",
        header,
        String::from_utf8(header.guid.to_ascii_hex_lower().to_vec()).expect("aaa")
    );

    return Ok(());
}
