use alloc::boxed::Box;

use crate::utils::oserror::OSError;
use crate::utils::port::{Port8Bit, Port16Bit};
use crate::{println,inline_if};

pub struct ATADrive {
    data_port: Port16Bit,
    error_port: Port8Bit,
    sector_count_port: Port8Bit,
    lba_low_port: Port8Bit,
    lba_mid_port: Port8Bit,
    lba_hi_port: Port8Bit,
    device_port: Port8Bit,
    command_port: Port8Bit,
    control_port: Port8Bit,
    master: bool,
    bytes_per_sector: usize,
    hpc: u16,
    sph: u16
}

impl ATADrive {
    pub fn new(port_base: u16, master: bool) -> Self {
        Self { data_port: Port16Bit::new(port_base)
            , error_port: Port8Bit::new(port_base + 1)
            , sector_count_port: Port8Bit::new(port_base + 2)
            , lba_low_port: Port8Bit::new(port_base + 3)
            , lba_mid_port: Port8Bit::new(port_base + 4)
            , lba_hi_port: Port8Bit::new(port_base + 5)
            , device_port: Port8Bit::new(port_base + 6)
            , command_port: Port8Bit::new(port_base + 7)
            , control_port: Port8Bit::new(port_base + 0x206)
            , master
            , bytes_per_sector: 512
            , hpc: 0
            , sph: 0
        }
    }

    pub fn get_hpc(&self) -> u16 {
        self.hpc
    }

    pub fn get_sph(&self) -> u16 {
        self.sph
    }

    pub async fn identify(&mut self) {
        self.device_port.write(inline_if!(self.master, 0xA0, 0xB0));
    
        self.control_port.write(0);

        self.device_port.write(0xA0);
    
        let mut status = self.command_port.read();

        if status == 0xFF {
            return;
        }

        self.device_port.write(inline_if!(self.master, 0xA0, 0xB0));

        self.sector_count_port.write(0);
        self.lba_low_port.write(0);
        self.lba_mid_port.write(0);
        self.lba_hi_port.write(0);
        self.command_port.write(0xEC);
        
        status = self.command_port.read();

        if status == 0 {
            return;
        }

        while ((status & 0x80) == 0x80) && ((status & 0x01) != 0x01) {
            status = self.command_port.read();
        }

        if (status & 0x01) != 0 {
            return;
        }
        
        let mut data: [u16; 256] = [0; 256];
        for i in 0..256 {
            data[i] = self.data_port.read();
        }
        
        let drive_size_in_sectors = (data[61] as u64) << 16 | data[60] as u64;
        
        let drive_size_in_gb_b10 = (drive_size_in_sectors as f64) * 512.0 / 1e9;
        
        let drive_size_in_gb_b2 = (drive_size_in_sectors * 512) / (1 << 30);

        println!("Drive size base 10: {}, base 2: {}", drive_size_in_gb_b10, drive_size_in_gb_b2);

        self.hpc = (data[6] & 0xFF) as u16;
        self.sph = (data[12] & 0xFF) as u16;
    }

    pub async fn write28(&mut self, sector: u32, data: &[u8], count: usize) -> Result<(), Box<OSError>> {
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(Box::new(OSError::new("Drive error")));
        }

        self.device_port.write((inline_if!(self.master, 0xE0, 0xF0) | ((sector & 0x0F000000) >> 24)) as u8); 
        self.error_port.write(0);
        self.sector_count_port.write(1);
        
        self.lba_low_port.write((sector & 0x000000FF) as u8);
        self.lba_mid_port.write(((sector & 0x0000FF00) >> 8) as u8);
        self.lba_hi_port.write(((sector & 0x00FF0000) >> 16) as u8);
        self.command_port.write(0x30);
        
        for i in (0..count).step_by(2) {
            let mut wdata = data[i] as u16;
            if i+1 < count {
                wdata |= (data[i+1] as u16) << 8; 
            }

            self.data_port.write(wdata);
        }
        
        for _i in ((count+(count % 2))..self.bytes_per_sector).step_by(2) {
            self.data_port.write(0x0000);
        }
        return Ok(());
    }

    pub async fn read28(&mut self, sector: u32, data: &mut [u8], count: usize) -> Result<(), Box<OSError>> { 
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(Box::new(OSError::new("Drive error")));
        }

        self.device_port.write((inline_if!(self.master, 0xE0, 0xF0) | ((sector & 0x0F000000) >> 24)) as u8); 
        self.error_port.write(0);
        self.sector_count_port.write(1);
        
        self.lba_low_port.write((sector & 0x000000FF) as u8);
        self.lba_mid_port.write(((sector & 0x0000FF00) >> 8) as u8);
        self.lba_hi_port.write(((sector & 0x00FF0000) >> 16) as u8);
        self.command_port.write(0x20);
        
        let mut status = self.command_port.read();

        while ((status & 0x80) == 0x80) && ((status & 0x01) != 0x01) {
            status = self.command_port.read();
        }

        if (status & 0x01) != 0 {
            return Err(Box::new(OSError::new("Drive error")));
        }
        
        for i in (0..count).step_by(2) {
            let wdata = self.data_port.read();
            
            data[i] = (wdata & 0xFF) as u8;
            if i+1 < count {
                data[i+1] = ((wdata >> 8) & 0xFF) as u8;
            }
        }
        
        for _i in ((count+(count % 2))..self.bytes_per_sector).step_by(2) {
            self.data_port.read();
        }
        return Ok(());
    }

    pub async fn flush(&mut self) -> Result<(), Box<OSError>> {
        self.device_port.write(inline_if!(self.master, 0xE0, 0xF0)); 
        self.command_port.write(0xE7);
    
        let mut status = self.command_port.read();
        while ((status & 0x80) == 0x80) && ((status & 0x01) != 0x01) {
            status = self.command_port.read();
        }

        if (status & 0x01) != 0 {
            return Err(Box::new(OSError::new("Drive error")));
        }
        return Ok(());
    }
}
