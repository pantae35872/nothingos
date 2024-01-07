use core::mem::size_of;
use alloc::boxed::Box;

use crate::driver::storage::CHS;
use crate::driver::storage::ata_driver::ATADrive;
use crate::utils::oserror::OSError;
use crate::{print, println};

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
struct PartitionTableEntry {
    bootable: u8,
    
    start_chs: CHS,
    partition_id: u8,
    end_chs: CHS,

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
    pub fn new() -> Result<Self, Box<OSError>>{
        Ok(Self { bootable: 0, start_chs: CHS::new(0, 0, 0)?, partition_id: 0, end_chs: CHS::new(0, 0, 0)?, start_lba: 0, length: 0 })
    }

    pub fn set_bootable(&mut self, bootable: bool) {
        self.bootable = bootable as u8;
    }

    pub fn set_start_chs(&mut self, chs: CHS) {
        self.start_chs = chs;
    }

    pub fn set_partition_id(&mut self, partition_id: u8) {
        self.partition_id = partition_id;
    }

    pub fn set_end_chs(&mut self, chs: CHS) {
        self.end_chs = chs;
    }

    pub fn set_start_lba(&mut self, start_lba: u32) {
        self.start_lba = start_lba;
    }

    pub fn set_length(&mut self, length: u32) {
        self.length = length;
    }
}

impl MasterBootRecord {
    pub fn new() -> Result<Self, Box<OSError>> {
        Ok(Self { bootloader: [0; 440], signature: 0, unused: 0, primary_partition: [PartitionTableEntry::new()?; 4], magicnumber: 0 })
    }
}

pub async fn read_partitions_ata(drive: &mut ATADrive) -> Result<(), Box<OSError>> {
    let mut mbr = MasterBootRecord::new()?; 
    let mbr_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(&mut mbr as *mut _ as *mut u8, size_of::<MasterBootRecord>())
    };
    drive.read28(0, mbr_bytes, size_of::<MasterBootRecord>()).await?;
    if mbr.magicnumber != 0xAA55 {
        return Err(Box::new(OSError::new("Drive is not formatted as mbr")));
    }
    
    let signature = mbr.signature;
    let reserved = mbr.unused;
    let magic = mbr.magicnumber;

    println!("signature: {}\nreserved: {}\nmagic: {:x}\n", signature, reserved, magic);

    //println!("{:#?}", mbr.primary_partition);

    /*for (i, partition) in mbr.primary_partition.iter().enumerate() {
        println!("Partition: {:#01x}", i & 0xFF);

        if partition.bootable == 0x80 {
            println!("bootable");
        } else {
            println!("not bootable. Type");
        }
        
        println!("c: {}, h: {}, s: {}",  partition.end_chs.get_cylinder(),
        partition.end_chs.get_head(), partition.end_chs.get_sector());
        
        println!("c: {}, h: {}, s: {}",  partition.start_chs.get_cylinder(),
        partition.start_chs.get_head(), partition.start_chs.get_sector());
        
        let start_lba = partition.start_lba;
        let e_lba = partition.length + partition.start_lba;
        println!("end lba: {} \nstart_lba: {}", e_lba - 1, start_lba);
    }*/

    return Ok(());
}

pub async fn format_ata(drive: &mut ATADrive) -> Result<(), Box<OSError>> {
    let mut mbr = MasterBootRecord::new()?; 
    let mbr_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(&mut mbr as *mut _ as *mut u8, size_of::<MasterBootRecord>())
    };
    mbr.magicnumber = 0xAA55;
    drive.write28(0, mbr_bytes, size_of::<MasterBootRecord>()).await?;
    drive.flush().await?;
    return Ok(());
}

pub async fn set_partitions_ata(drive: &mut ATADrive, partition_id: u8, partition_number: usize, start_lba: u32, length: u32, bootable: bool) -> Result<(), Box<OSError>> {
    let mut mbr = MasterBootRecord::new()?; 
    let mbr_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(&mut mbr as *mut _ as *mut u8, size_of::<MasterBootRecord>())
    };
    drive.read28(0, mbr_bytes, size_of::<MasterBootRecord>()).await?;
    if mbr.magicnumber != 0xAA55 {
        return Err(Box::new(OSError::new("Drive is not formatted as mbr")));
    }
    
    let partition = &mut mbr.primary_partition[partition_number];
    partition.set_bootable(bootable);
    partition.set_start_lba(start_lba);
    partition.set_length(length);
    partition.set_partition_id(partition_id);
    let start_chs = CHS::from_lba(start_lba)?;
    partition.set_start_chs(start_chs);
    let end_chs = CHS::from_lba((start_lba + length) - 1)?;
    partition.set_end_chs(end_chs);

    drive.write28(0, mbr_bytes, size_of::<MasterBootRecord>()).await?;
    drive.flush().await?;
    return Ok(());
}
