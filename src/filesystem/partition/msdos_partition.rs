use alloc::boxed::Box;
use core::mem::size_of;

use crate::driver::storage::{Drive, CHS};
use crate::utils::oserror::OSError;

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct PartitionTableEntry {
    bootable: u8,

    start_chs: CHS,
    partition_id: u8,
    end_chs: CHS,

    start_lba: u32,
    end_lba: u32,
}

#[repr(packed)]
struct MasterBootRecord {
    bootloader: [u8; 440],
    signature: u32,
    unused: u16,

    primary_partition: [PartitionTableEntry; 4],
    magicnumber: u16,
}

impl PartitionTableEntry {
    pub fn new() -> Result<Self, Box<OSError>> {
        Ok(Self {
            bootable: 0,
            start_chs: CHS::new(0, 0, 0)?,
            partition_id: 0,
            end_chs: CHS::new(0, 0, 0)?,
            start_lba: 0,
            end_lba: 0,
        })
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

    pub fn set_end_lba(&mut self, end_lba: u32) {
        self.end_lba = end_lba;
    }

    pub fn get_bootable(&self) -> bool {
        self.bootable != 0
    }

    pub fn get_start_chs(&self) -> CHS {
        self.start_chs
    }

    pub fn get_end_chs(&self) -> CHS {
        self.end_chs
    }

    pub fn get_id(&self) -> u8 {
        self.partition_id
    }

    pub fn get_start_lba(&self) -> u32 {
        self.start_lba
    }

    pub fn get_end_lba(&self) -> u32 {
        self.end_lba
    }
}

impl MasterBootRecord {
    pub fn new() -> Result<Self, Box<OSError>> {
        Ok(Self {
            bootloader: [0; 440],
            signature: 0,
            unused: 0,
            primary_partition: [PartitionTableEntry::new()?; 4],
            magicnumber: 0,
        })
    }
}

pub struct MSDosPartition<'a, T>
where
    T: Drive,
{
    master_boot_record: MasterBootRecord,
    drive: &'a mut T,
}

impl<'a, T: Drive> MSDosPartition<'a, T> {
    pub async fn new(drive: &'a mut T) -> Result<Self, Box<OSError>> {
        let mut msdos_partition = Self {
            master_boot_record: MasterBootRecord::new()?,
            drive,
        };

        msdos_partition.load_mbr().await?;

        return Ok(msdos_partition);
    }

    pub async fn load_mbr(&mut self) -> Result<(), Box<OSError>> {
        let mbr_bytes: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(
                &mut self.master_boot_record as *mut _ as *mut u8,
                size_of::<MasterBootRecord>(),
            )
        };

        self.drive
            .read(0, mbr_bytes, size_of::<MasterBootRecord>())
            .await?;

        if self.master_boot_record.magicnumber != 0xAA55 {
            return Err(Box::new(OSError::new("Not valid ms dos drive.")));
        }

        Ok(())
    }

    pub async fn save_mbr(&mut self) -> Result<(), Box<OSError>> {
        let mbr_bytes: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(
                &mut self.master_boot_record as *mut _ as *mut u8,
                size_of::<MasterBootRecord>(),
            )
        };
        self.drive
            .write(0, mbr_bytes, size_of::<MasterBootRecord>())
            .await?;
        Ok(())
    }

    pub async fn format(&mut self) -> Result<(), Box<OSError>> {
        self.master_boot_record = MasterBootRecord::new()?;
        self.master_boot_record.magicnumber = 0xAA55;
        self.save_mbr().await?;
        return Ok(());
    }

    pub async fn read_partition(
        &mut self,
        partition_number: usize,
    ) -> Result<PartitionTableEntry, Box<OSError>> {
        if partition_number >= 4 {
            return Err(Box::new(OSError::new(
                "Partition number cannot be more than 3 starts at 0",
            )));
        }

        Ok(self.master_boot_record.primary_partition[partition_number])
    }

    pub async fn set_partition(
        &mut self,
        partition_id: u8,
        partition_number: usize,
        start_lba: u32,
        end_lba: u32,
        bootable: bool,
    ) -> Result<(), Box<OSError>> {
        self.load_mbr().await?;
        if self.master_boot_record.magicnumber != 0xAA55 {
            return Err(Box::new(OSError::new("Drive is not formatted as mbr")));
        }

        let partition = &mut self.master_boot_record.primary_partition[partition_number];
        partition.set_bootable(bootable);
        partition.set_start_lba(start_lba);
        partition.set_end_lba(end_lba);
        partition.set_partition_id(partition_id);
        let start_chs = CHS::from_lba(start_lba)?;
        partition.set_start_chs(start_chs);
        let end_chs = CHS::from_lba((start_lba + end_lba) - 1)?;
        partition.set_end_chs(end_chs);

        self.save_mbr().await?;
        return Ok(());
    }
}
