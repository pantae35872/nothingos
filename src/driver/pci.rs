use crate::utils::port::Port32Bit;

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

    pub fn read(&mut self, bus: &u16, device: &u16, function: &u16, offset: &u32) -> u32 {
        let id: u32 = 0x1 << 31
            | ((*bus as u32 & 0xFF) << 16)
            | ((*device as u32 & 0x1F) << 11)
            | ((*function as u32 & 0x07) << 8)
            | (offset & 0xFC);
        self.command_port.write(&id);

        return self.data_port.read() >> (8 * (*offset as u32 % 4));
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
