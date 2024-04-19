pub mod ahci_driver;
pub mod ata_driver;

use core::{f64, future};

use alloc::boxed::Box;

use crate::utils::floorf64;
use crate::utils::oserror::OSError;
use crate::MemoryController;

pub const MAXSCT: f64 = 63.0;
pub const MAXHD: f64 = 255.0;

#[derive(Debug, Clone, Copy)]
pub struct CHS {
    head: u8,
    sector: u8,
    cylinder: u8,
}

impl CHS {
    pub fn new(h: u8, s: u8, c: u16) -> Result<Self, Box<OSError>> {
        let mut chs = Self {
            head: 0,
            sector: 0,
            cylinder: 0,
        };
        chs.set_head(h);
        chs.set_sector(s)?;
        chs.set_cylinder(c)?;
        Ok(chs)
    }

    pub fn get_cylinder(&self) -> u16 {
        return ((self.sector as u16 & 0b11000000) << 2) | self.cylinder as u16;
    }

    pub fn get_head(&self) -> u8 {
        self.head
    }

    pub fn get_sector(&self) -> u8 {
        return self.sector & 0b00111111;
    }

    pub fn set_cylinder(&mut self, cylinder: u16) -> Result<(), Box<OSError>> {
        if cylinder > 1023 {
            return Err(Box::new(OSError::new("Cylinder cannot be more than 1023")));
        }

        self.sector = (((cylinder >> 2) & 0b11000000) | self.sector as u16) as u8;
        self.cylinder = (cylinder & 0b11111111) as u8;
        return Ok(());
    }

    pub fn set_head(&mut self, head: u8) {
        self.head = head;
    }

    pub fn set_sector(&mut self, sector: u8) -> Result<(), Box<OSError>> {
        if sector > 63 {
            return Err(Box::new(OSError::new("Drive is not formatted as mbr")));
        }

        self.sector = (sector & 0b00111111) | self.sector;
        Ok(())
    }

    pub fn from_lba(lba: u32) -> Result<Self, Box<OSError>> {
        let mut cylinder = floorf64((lba as f64) / (MAXSCT * MAXHD as f64));
        let mut work1 = cylinder * (MAXSCT * MAXHD);
        work1 = (lba as f64) - work1;
        let hd = floorf64(work1 / MAXSCT);
        let sct = work1 - hd * MAXSCT + 1.0;

        if cylinder > 1023.0 {
            cylinder = 1023.0;
        }

        Ok(Self::new(hd as u8, sct as u8, cylinder as u16)?)
    }
}

pub trait Drive {
    fn write(
        &mut self,
        from_sector: u64,
        data: &[u8],
        count: usize,
    ) -> impl future::Future<Output = Result<(), Box<OSError>>> + Send;

    fn read(
        &mut self,
        from_sector: u64,
        data: &mut [u8],
        count: usize,
    ) -> impl future::Future<Output = Result<(), Box<OSError>>> + Send;

    fn lba_end(&self) -> u64;
}

pub fn init(frame_allocator: &mut MemoryController) {
    ahci_driver::init(frame_allocator);
}
