use crate::port::{Port8Bit, Port16Bit};
use crate::{println, print};

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
    master: bool
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
        }
    }

    pub fn identify(&mut self) {
        if self.master {
            self.device_port.write(0xA0);
        } else {
            self.device_port.write(0xB0);
        }
    
        self.control_port.write(0);

        self.device_port.write(0xA0);
    
        let mut status = self.command_port.read();

        if status == 0xFF {
            return;
        }
        if self.master {
            self.device_port.write(0xA0);
        } else {
            self.device_port.write(0xB0);
        }

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
            println!("DRIVE ERROR");
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
    }
}
