use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::utils::port::Port32Bit;

pub static DRIVER: OnceCell<Mutex<PCIControler>> = OnceCell::uninit();

pub struct PCIControler {
    data_port: Port32Bit,
    command_port: Port32Bit,
}

impl PCIControler {
    pub fn new() -> Self {
        Self {
            data_port: Port32Bit::new(0xCFC),
            command_port: Port32Bit::new(0xCF8),
        }
    }

    pub fn read_config(&mut self, bus: &u16, slot: &u16, function: &u16, offset: &u16) -> u32 {
        let address: u32 = ((*bus as u32) << 16)
            | ((*slot as u32) << 11)
            | ((*function as u32) << 8)
            | ((*offset as u32) & 0xFC)
            | 0x80000000;

        self.command_port.write(&address);

        return self.data_port.read() >> ((offset & 2) * 8) & 0xFFFF;
    }

    pub fn read(&mut self, bus: &u16, slot: &u16, function: &u16, offset: &u16) -> u32 {
        let address: u32 = ((*bus as u32) << 16)
            | ((*slot as u32) << 11)
            | ((*function as u32) << 8)
            | ((*offset as u32) & 0xFC)
            | 0x80000000;
        self.command_port.write(&address);

        return self.data_port.read();
    }

    pub fn write(
        &mut self,
        bus: &u16,
        device: &u16,
        function: &u16,
        registeroffset: &u32,
        value: &u32,
    ) {
        let id: u32 = 0x1 << 31
            | ((*bus as u32 & 0xFF) << 16)
            | ((*device as u32 & 0x1F) << 11)
            | ((*function as u32 & 0x07) << 8)
            | (registeroffset & 0xFC);

        self.command_port.write(&id);
        self.data_port.write(value);
    }
}

pub fn init() {
    DRIVER.init_once(|| Mutex::from(PCIControler::new()));
}
