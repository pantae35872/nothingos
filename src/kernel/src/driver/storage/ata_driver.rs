use core::error::Error;
use core::fmt::Display;

use crate::inline_if;
use crate::utils::port::{Port16Bit, Port8Bit};

use super::Drive;

#[derive(Debug)]
pub enum AtaDriveError {
    InvalidByteCount(usize),
    DriveError(u8),
}

impl Display for AtaDriveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidByteCount(count) => write!(
                f,
                "Trying to read once with byte count that is more than 512. value: {}",
                count
            ),
            Self::DriveError(status) => write!(f, "Drive error with status {}", status),
        }
    }
}

impl Error for AtaDriveError {}

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
    lba_end: u64,
}

impl ATADrive {
    pub fn new(port_base: u16, master: bool) -> Self {
        Self {
            data_port: Port16Bit::new(port_base),
            error_port: Port8Bit::new(port_base + 1),
            sector_count_port: Port8Bit::new(port_base + 2),
            lba_low_port: Port8Bit::new(port_base + 3),
            lba_mid_port: Port8Bit::new(port_base + 4),
            lba_hi_port: Port8Bit::new(port_base + 5),
            device_port: Port8Bit::new(port_base + 6),
            command_port: Port8Bit::new(port_base + 7),
            control_port: Port8Bit::new(port_base + 0x206),
            master,
            bytes_per_sector: 512,
            lba_end: 0,
        }
    }

    pub fn identify(&mut self) {
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
        loop {
            status = self.command_port.read();
            if !(((status & 0x80) == 0x80) && ((status & 0x01) != 0x01)) {
                break;
            }
        }

        status = self.command_port.read();

        if (status & 0x01) != 0 {
            return;
        }

        let mut data: [u16; 256] = [0; 256];
        for i in 0..256 {
            data[i] = self.data_port.read();
        }

        let lba_end_low = u64::from(data[100]);
        let lba_end_high = u64::from(data[101]);

        self.lba_end = ((lba_end_high << 16) | lba_end_low) - 1;
    }

    fn write_once(&mut self, sector: u64, data: &[u8], count: usize) -> Result<(), AtaDriveError> {
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(AtaDriveError::InvalidByteCount(count));
        }

        self.device_port
            .write((inline_if!(self.master, 0xE0, 0xF0) | ((sector & 0x0F000000) >> 24)) as u8);
        self.error_port.write(0);
        self.sector_count_port.write(1);

        self.lba_low_port.write((sector & 0x000000FF) as u8);
        self.lba_mid_port.write(((sector & 0x0000FF00) >> 8) as u8);
        self.lba_hi_port.write(((sector & 0x00FF0000) >> 16) as u8);
        self.command_port.write(0x30);

        for i in (0..count).step_by(2) {
            let mut wdata = data[i] as u16;
            if i + 1 < count {
                wdata |= (data[i + 1] as u16) << 8;
            }

            self.data_port.write(wdata);
        }

        for _i in ((count + (count % 2))..self.bytes_per_sector).step_by(2) {
            self.data_port.write(0x0000);
        }
        self.flush()?;
        return Ok(());
    }

    fn read_once(
        &mut self,
        sector: u64,
        data: &mut [u8],
        count: usize,
    ) -> Result<(), AtaDriveError> {
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(AtaDriveError::InvalidByteCount(count));
        }

        self.device_port
            .write((inline_if!(self.master, 0xE0, 0xF0) | ((sector & 0x0F000000) >> 24)) as u8);
        self.error_port.write(0);
        self.sector_count_port.write(1);

        self.lba_low_port.write((sector & 0x000000FF) as u8);
        self.lba_mid_port.write(((sector & 0x0000FF00) >> 8) as u8);
        self.lba_hi_port.write(((sector & 0x00FF0000) >> 16) as u8);
        self.command_port.write(0x20);

        let mut status;
        loop {
            status = self.command_port.read();
            if !(((status & 0x80) == 0x80) && ((status & 0x01) != 0x01)) {
                break;
            }
        }
        status = self.command_port.read();

        if (status & 0x01) != 0 {
            return Err(AtaDriveError::DriveError(status));
        }

        for i in (0..count).step_by(2) {
            let wdata = self.data_port.read();

            data[i] = (wdata & 0xFF) as u8;
            if i + 1 < count {
                data[i + 1] = ((wdata >> 8) & 0xFF) as u8;
            }
        }

        for _i in ((count + (count % 2))..self.bytes_per_sector).step_by(2) {
            self.data_port.read();
        }
        return Ok(());
    }

    pub fn flush(&mut self) -> Result<(), AtaDriveError> {
        self.device_port.write(inline_if!(self.master, 0xE0, 0xF0));
        self.command_port.write(0xE7);
        let mut status;
        loop {
            status = self.command_port.read();
            if !(((status & 0x80) == 0x80) && ((status & 0x01) != 0x01)) {
                break;
            }
        }

        status = self.command_port.read();

        if (status & 0x01) != 0 {
            return Err(AtaDriveError::DriveError(status));
        }
        return Ok(());
    }
}

impl Drive for ATADrive {
    type Error = AtaDriveError;

    fn lba_end(&self) -> u64 {
        self.lba_end
    }

    fn write(&mut self, from_sector: u64, buffer: &[u8], count: usize) -> Result<(), Self::Error> {
        for i in 0..count {
            self.write_once(
                from_sector + i as u64,
                &buffer[(512 * i)..(512 * (i + 1))],
                512,
            )?;
        }
        return Ok(());
    }

    fn read(
        &mut self,
        from_sector: u64,
        buffer: &mut [u8],
        count: usize,
    ) -> Result<(), Self::Error> {
        for i in 0..count {
            self.read_once(
                from_sector + i as u64,
                &mut buffer[(512 * i)..(512 * (i + 1))],
                512,
            )?;
        }
        return Ok(());
    }
}
