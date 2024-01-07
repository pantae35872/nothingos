use core::mem::size_of;
use crate::driver::storage::ata_driver::ATADrive;
use crate::{print, println};

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
struct PartitionTableEntry {
    bootable: u8,

    start_head: u8,
    start_sector: u8,
    start_cylinder: u8,

    partition_id: u8,

    end_head: u8,
    end_sector: u8,
    end_cylinder: u8,

    start_lba: u32,
    length: u32,
}

#[repr(packed)]
#[derive(Debug)]
struct MasterBootRecord {
    bootloader: [u8; 440],
    signature: u32,
    unused: u16,

    primary_partition: [PartitionTableEntry; 4],
    magicnumber: u16,
}

impl PartitionTableEntry {
    pub fn new() -> Self {
        Self { bootable: 0, start_head: 0, start_sector: 6, start_cylinder: 10, partition_id: 0, end_head: 0, end_sector: 6, end_cylinder: 10, start_lba: 0, length: 0 }
    }
}

impl MasterBootRecord {
    pub fn new() -> Self {
        Self { bootloader: [0; 440], signature: 0, unused: 0, primary_partition: [PartitionTableEntry::new(); 4], magicnumber: 0 }
    }
}

pub async fn read_partitions_ata(drive: &mut ATADrive) {
    let mut mbr = MasterBootRecord::new(); 
    let mbr_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(&mut mbr as *mut _ as *mut u8, size_of::<MasterBootRecord>())
    };
    drive.read28(0, mbr_bytes, size_of::<MasterBootRecord>()).await;
    if mbr.magicnumber != 0xAA55 {
        return;
    }

    for i in 0..4 {
        println!("Partition: {:#01x}", i & 0xFF);

        if mbr.primary_partition[i].bootable == 0x80 {
            println!("bootable");
        } else {
            println!("not bootable. Type");
        }

        println!("{:#01x}", mbr.primary_partition[i].partition_id);
    }
}
