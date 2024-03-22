use core::mem::size_of;
use core::slice;

use alloc::boxed::Box;
use alloc::string::String;
use crc::{crc32, Hasher32};
use uguid::Guid;
use uuid::Uuid;

use crate::driver::storage::{Drive, CHS};
use crate::utils::floorf64;
use crate::utils::oserror::OSError;
use crate::{print, println};

use super::msdos_partition::MSDosPartition;

#[derive(Debug, Clone, Copy)]
struct ProtectiveMasterBootRecord {
    bootable: bool,
    start_chs: CHS,
    os_type: u8,
    end_chs: CHS,
    start_lba: u32,
    end_lba: u32,
}

impl ProtectiveMasterBootRecord {
    pub fn new() -> Result<Self, Box<OSError>> {
        Ok(ProtectiveMasterBootRecord {
            bootable: false,
            start_chs: CHS::new(0, 0, 0)?,
            os_type: 0,
            end_chs: CHS::new(0, 0, 0)?,
            start_lba: 0,
            end_lba: 0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct PartitionTableHeader {
    signature: [u8; 8],
    gpt_revision: u32,
    header_size: u32,
    checksum: u32,
    reserved: u32,
    header_lba: u64,
    backup_header_lba: u64,
    start_usable: u64,
    end_usable: u64,
    guid: Guid,
    start_partition_entry_lba: u64,
    number_partition_entries: u32,
    partition_entry_size: u32,
    partition_entry_array_checksum: u32,
    reserved_zero: [u8; 420],
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct PartitionEntry {
    partition_type: Guid,
    guid: Guid,
    start_lba: u64,
    end_lba: u64,
    attributes: u64,
    partition_name: [u8; 72],
}

impl PartitionEntry {
    pub fn get_partition_name(&self) -> String {
        String::from_utf16le(&self.partition_name).expect("Error")
    }
}

impl PartitionEntry {
    pub fn new() -> Self {
        Self {
            partition_type: Guid::ZERO,
            guid: Guid::ZERO,
            start_lba: 0,
            end_lba: 0,
            attributes: 0,
            partition_name: [0; 72],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
struct PartitionEntries {
    entries: [PartitionEntry; 4],
}

impl PartitionTableHeader {
    pub fn new() -> Self {
        Self {
            signature: [0u8; 8],
            gpt_revision: 0,
            header_size: 0,
            checksum: 0,
            reserved: 0,
            header_lba: 0,
            backup_header_lba: 0,
            start_usable: 0,
            end_usable: 0,
            guid: Guid::ZERO,
            start_partition_entry_lba: 0,
            number_partition_entries: 0,
            partition_entry_size: 0,
            partition_entry_array_checksum: 0,
            reserved_zero: [0u8; 420],
        }
    }
}

impl PartitionEntries {
    pub fn new() -> Self {
        Self {
            entries: [PartitionEntry::new(); 4],
        }
    }
}

pub struct GPTPartitions<'a, T>
where
    T: Drive,
{
    protective_master_boot_record: ProtectiveMasterBootRecord,
    partition_table_header: PartitionTableHeader,
    partition_entries: [PartitionEntries; 32],
    drive: &'a mut T,
    entries_per_sector: u32,
    sector_number_entries: usize,
}

impl<'a, T: Drive> GPTPartitions<'a, T> {
    pub async fn new(drive: &'a mut T) -> Result<Self, Box<OSError>> {
        Ok(Self {
            protective_master_boot_record: ProtectiveMasterBootRecord::new()?,
            partition_table_header: PartitionTableHeader::new(),
            partition_entries: [PartitionEntries::new(); 32],
            drive,
            entries_per_sector: 0,
            sector_number_entries: 0,
        })
    }

    pub async fn format(&mut self) -> Result<(), Box<OSError>> {
        MSDosPartition::new(self.drive).await?.format().await?;
        self.protective_master_boot_record.bootable = false;
        self.protective_master_boot_record.start_lba = 1;
        self.protective_master_boot_record.start_chs = CHS::from_lba(1)?;
        self.protective_master_boot_record.os_type = 0xEE;
        self.protective_master_boot_record.end_lba =
            self.drive.lba_end().try_into().unwrap_or(0xFFFFFFFF);
        self.protective_master_boot_record.end_chs =
            CHS::from_lba(self.drive.lba_end().try_into().unwrap_or(0xFFFFFFFF))?;

        self.partition_table_header.signature = [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54];
        self.partition_table_header.reserved_zero = [0; 420];
        self.partition_table_header.reserved = 0;
        self.partition_table_header.number_partition_entries = 128;
        self.partition_table_header.backup_header_lba = self.drive.lba_end();
        self.partition_table_header.header_lba = 1;
        self.partition_table_header.guid = Guid::from_bytes(*Uuid::new_v4().as_bytes());
        self.partition_table_header.start_usable = 34;
        self.partition_table_header.end_usable = self.drive.lba_end() - 33;
        self.partition_table_header.header_size = 0x5C;
        self.partition_table_header.gpt_revision = 0x00010000;
        self.partition_table_header.partition_entry_size = 0x80;
        self.partition_table_header.start_partition_entry_lba = 2;
        self.entries_per_sector = 4;
        self.sector_number_entries = 32;

        self.save_gpt().await?;

        Ok(())
    }

    pub async fn read_partition(&mut self, number: usize) -> Result<PartitionEntry, Box<OSError>> {
        self.load_gpt().await?;
        let entries_lba = (floorf64((number as f64 / 4.0) + 0.25) - 1.0) as usize;
        let mut entry_index = number % 4;
        if entry_index == 0 {
            entry_index = 4;
        }

        Ok(self.partition_entries[entries_lba].entries[entry_index - 1])
    }

    pub async fn set_partiton(
        &mut self,
        drive_number: usize,
        partition_type: &Guid,
        start_lba: u64,
        end_lba: u64,
        attributes: u64,
        partition_name: &[u8; 72],
    ) -> Result<(), Box<OSError>> {
        self.load_gpt().await?;

        let entries_lba = (floorf64((drive_number as f64 / 4.0) + 0.25) - 1.0) as usize;
        let mut entry_index = drive_number % 4;
        if entry_index == 0 {
            entry_index = 4;
        }

        let entry: &mut PartitionEntry =
            &mut self.partition_entries[entries_lba].entries[entry_index - 1];

        entry.partition_type = *partition_type;
        entry.start_lba = start_lba;
        entry.end_lba = end_lba;
        entry.attributes = attributes;
        entry.partition_name = *partition_name;
        entry.guid = Guid::from_bytes(*Uuid::new_v4().as_bytes());

        self.save_gpt().await?;

        Ok(())
    }

    async fn load_gpt(&mut self) -> Result<(), Box<OSError>> {
        let mut mbr = MSDosPartition::new(self.drive).await?;
        mbr.load_mbr().await?;
        let par = mbr.read_partition(0).await?;
        self.protective_master_boot_record.start_lba = par.get_start_lba();
        self.protective_master_boot_record.start_chs = par.get_start_chs();
        self.protective_master_boot_record.os_type = par.get_id();
        self.protective_master_boot_record.end_chs = par.get_end_chs();
        self.protective_master_boot_record.bootable = par.get_bootable();
        self.protective_master_boot_record.end_lba = par.get_end_lba();

        let header_bytes: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_table_header as *mut _ as *mut u8,
                size_of::<PartitionTableHeader>(),
            )
        };

        self.drive.read(1, header_bytes, 1).await?;

        self.entries_per_sector = 512 / self.partition_table_header.partition_entry_size;
        self.sector_number_entries = (self.partition_table_header.number_partition_entries
            / self.entries_per_sector) as usize;

        let entries_bytes: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_entries as *mut _ as *mut u8,
                size_of::<PartitionEntries>() * self.sector_number_entries,
            )
        };

        self.drive
            .read(
                self.partition_table_header.start_partition_entry_lba,
                entries_bytes,
                self.sector_number_entries,
            )
            .await?;

        self.validate().await?;
        Ok(())
    }

    async fn save_gpt(&mut self) -> Result<(), Box<OSError>> {
        let mut crc32 = crc32::Digest::new(crc32::IEEE);
        let mut mbr = MSDosPartition::new(self.drive).await?;
        mbr.set_partition(
            self.protective_master_boot_record.os_type,
            0,
            self.protective_master_boot_record.start_lba,
            self.protective_master_boot_record.end_lba,
            self.protective_master_boot_record.bootable,
        )
        .await?;

        let header_bytes: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_table_header as *mut _ as *mut u8,
                size_of::<PartitionTableHeader>(),
            )
        };
        let entries_bytes: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_entries as *mut _ as *mut u8,
                size_of::<PartitionEntries>() * self.sector_number_entries,
            )
        };

        crc32.write(&entries_bytes);
        self.partition_table_header.partition_entry_array_checksum = crc32.sum32();
        crc32.reset();
        self.partition_table_header.checksum = 0;
        crc32.write(&header_bytes[0..0x5c]);
        self.partition_table_header.checksum = crc32.sum32();

        self.drive
            .write(1, header_bytes, size_of::<PartitionTableHeader>() / 512)
            .await?;

        let start_lba = self.partition_table_header.start_partition_entry_lba;

        self.drive
            .write(start_lba, entries_bytes, self.sector_number_entries)
            .await?;

        self.drive
            .write(
                self.drive.lba_end(),
                header_bytes,
                size_of::<PartitionTableHeader>() / 512,
            )
            .await?;

        let start_lba = self.drive.lba_end() - self.sector_number_entries as u64;
        self.drive
            .write(start_lba, entries_bytes, self.sector_number_entries)
            .await?;

        Ok(())
    }

    pub async fn validate(&mut self) -> Result<(), Box<OSError>> {
        let mut crc32 = crc32::Digest::new(crc32::IEEE);
        let header_bytes: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_table_header as *mut _ as *mut u8,
                size_of::<PartitionTableHeader>(),
            )
        };
        let entries_bytes: &[u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut self.partition_entries as *mut _ as *mut u8,
                size_of::<PartitionEntries>() * self.sector_number_entries,
            )
        };

        let before_check = self.partition_table_header.checksum;
        self.partition_table_header.checksum = 0;
        crc32.write(&header_bytes[0..0x5c]);
        self.partition_table_header.checksum = before_check;
        if self.partition_table_header.checksum != crc32.sum32()
            || self.partition_table_header.signature
                != [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54]
        {
            println!("drive header is corrupt try recover from backup");
            self.drive
                .read(
                    self.drive.lba_end(),
                    header_bytes,
                    size_of::<PartitionTableHeader>() / 512,
                )
                .await?;
            crc32.reset();
            crc32.write(&header_bytes[0..0x5c]);
            if self.partition_table_header.checksum == crc32.sum32()
                || self.partition_table_header.signature
                    == [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54]
            {
                println!("Header backup is valid, restoring from backup");
                self.drive
                    .write(1, header_bytes, size_of::<PartitionTableHeader>() / 512);
            } else {
                return Err(Box::new(OSError::new(
                    "Your header backup is not valid, drive is fully corrupted",
                )));
            }
        }
        crc32.reset();
        crc32.write(&entries_bytes);
        if self.partition_table_header.partition_entry_array_checksum != crc32.sum32() {
            println!("drive entries is corrupt try recover from backup");
            let entries_bytes: &mut [u8] = unsafe {
                slice::from_raw_parts_mut(
                    &mut self.partition_entries as *mut _ as *mut u8,
                    size_of::<PartitionEntries>() * self.sector_number_entries,
                )
            };
            let start_lba = self.drive.lba_end()
                - ((self.partition_table_header.number_partition_entries / self.entries_per_sector)
                    as u64
                    + 1);

            self.drive
                .read(start_lba, entries_bytes, self.sector_number_entries)
                .await?;
            crc32.reset();
            crc32.write(&entries_bytes);

            if self.partition_table_header.partition_entry_array_checksum == crc32.sum32() {
                println!("Entries backup is valid, restoring from backup");
                let start_lba = self.partition_table_header.start_partition_entry_lba;
                self.drive
                    .write(start_lba, entries_bytes, self.sector_number_entries);
            } else {
                return Err(Box::new(OSError::new(
                    "Your entries backup is not valid, drive is fully corrupted",
                )));
            }
        }
        Ok(())
    }
}
