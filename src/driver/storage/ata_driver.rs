use core::task::Poll;

use alloc::boxed::Box;
use futures_util::Future;

use crate::utils::oserror::OSError;
use crate::utils::port::{Port16Bit, Port8Bit};
use crate::{inline_if, println};

use super::Drive;

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

        DriveAsync::new(&self.command_port).await;

        status = self.command_port.read();

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

        println!(
            "Drive size base 10: {}, base 2: {}",
            drive_size_in_gb_b10, drive_size_in_gb_b2
        );

        let lba_end_low = u64::from(data[100]);
        let lba_end_high = u64::from(data[101]);

        self.lba_end = ((lba_end_high << 16) | lba_end_low) - 1;
    }

    pub async fn flush(&mut self) -> Result<(), Box<OSError>> {
        self.device_port.write(inline_if!(self.master, 0xE0, 0xF0));
        self.command_port.write(0xE7);

        DriveAsync::new(&self.command_port).await;

        let status = self.command_port.read();

        if (status & 0x01) != 0 {
            return Err(Box::new(OSError::new("Drive error")));
        }
        return Ok(());
    }
}

struct DriveAsync<'a> {
    command_port: &'a Port8Bit,
}

impl<'a> DriveAsync<'a> {
    pub fn new(command_port: &'a Port8Bit) -> Self {
        Self { command_port }
    }
}

impl<'a> Future for DriveAsync<'a> {
    type Output = ();

    fn poll(self: core::pin::Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<()> {
        let status = self.command_port.read();
        if ((status & 0x80) == 0x80) && ((status & 0x01) != 0x01) {
            return Poll::Pending;
        } else {
            return Poll::Ready(());
        }
    }
}

impl Drive for ATADrive {
    fn lba_end(&self) -> u64 {
        self.lba_end
    }

    async fn write(&mut self, sector: u64, data: &[u8], count: usize) -> Result<(), Box<OSError>> {
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(Box::new(OSError::new("Drive error")));
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
        self.flush().await?;
        return Ok(());
    }

    async fn read(
        &mut self,
        sector: u64,
        data: &mut [u8],
        count: usize,
    ) -> Result<(), Box<OSError>> {
        if (sector & 0xF0000000) != 0 || count > self.bytes_per_sector {
            return Err(Box::new(OSError::new("Drive error")));
        }

        self.device_port
            .write((inline_if!(self.master, 0xE0, 0xF0) | ((sector & 0x0F000000) >> 24)) as u8);
        self.error_port.write(0);
        self.sector_count_port.write(1);

        self.lba_low_port.write((sector & 0x000000FF) as u8);
        self.lba_mid_port.write(((sector & 0x0000FF00) >> 8) as u8);
        self.lba_hi_port.write(((sector & 0x00FF0000) >> 16) as u8);
        self.command_port.write(0x20);

        DriveAsync::new(&self.command_port).await;

        let status = self.command_port.read();

        if (status & 0x01) != 0 {
            return Err(Box::new(OSError::new("Drive error")));
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
}
