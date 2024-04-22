pub struct BufferReader<'a> {
    buffer: &'a [u8],
    index: usize,
}

pub enum Endian {
    BigEndian,
    LittleEndian,
}

#[derive(Debug)]
pub enum BufferReadError {
    OutOfRange,
}

impl<'a> BufferReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, index: 0 }
    }

    pub fn read_byte(&mut self) -> Result<u8, BufferReadError> {
        if self.index >= self.buffer.len() {
            return Err(BufferReadError::OutOfRange);
        }
        let value = self.buffer[self.index];
        self.index += 1;
        return Ok(value);
    }

    pub fn get_index(&self) -> usize {
        return self.index;
    }

    pub fn read_bytes(&mut self, amount: usize) -> Result<&'a [u8], BufferReadError> {
        if self.index + amount > self.buffer.len() {
            return Err(BufferReadError::OutOfRange);
        }
        let result = &self.buffer[self.index..(self.index + amount)];
        self.index += amount;
        Ok(result)
    }

    pub fn read_i32(&mut self, endian: Endian) -> Result<i32, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(i32::from_be_bytes(
                self.read_bytes(4)?[0..4]
                    .try_into()
                    .expect("The length of read bytes is somehow not 4"),
            )),
            Endian::LittleEndian => Ok(i32::from_le_bytes(
                self.read_bytes(4)?[0..4]
                    .try_into()
                    .expect("The length of read bytes is somehow not 4"),
            )),
        }
    }
    pub fn read_u32(&mut self, endian: Endian) -> Result<u32, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(u32::from_be_bytes(
                self.read_bytes(4)?[0..4]
                    .try_into()
                    .expect("The length of read bytes is somehow not 4"),
            )),
            Endian::LittleEndian => Ok(u32::from_le_bytes(
                self.read_bytes(4)?[0..4]
                    .try_into()
                    .expect("The length of read bytes is somehow not 4"),
            )),
        }
    }
    pub fn read_i64(&mut self, endian: Endian) -> Result<i64, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(i64::from_be_bytes(
                self.read_bytes(8)?[0..8]
                    .try_into()
                    .expect("The length of read bytes is somehow not 8"),
            )),
            Endian::LittleEndian => Ok(i64::from_le_bytes(
                self.read_bytes(8)?[0..8]
                    .try_into()
                    .expect("The length of read bytes is somehow not 8"),
            )),
        }
    }
    pub fn read_u64(&mut self, endian: Endian) -> Result<u64, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(u64::from_be_bytes(
                self.read_bytes(8)?[0..8]
                    .try_into()
                    .expect("The length of read bytes is somehow not 8"),
            )),
            Endian::LittleEndian => Ok(u64::from_le_bytes(
                self.read_bytes(8)?[0..8]
                    .try_into()
                    .expect("The length of read bytes is somehow not 8"),
            )),
        }
    }

    pub fn read_u16(&mut self, endian: Endian) -> Result<u16, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(u16::from_be_bytes(
                self.read_bytes(2)?[0..2]
                    .try_into()
                    .expect("The length of read bytes is somehow not 2"),
            )),
            Endian::LittleEndian => Ok(u16::from_le_bytes(
                self.read_bytes(2)?[0..2]
                    .try_into()
                    .expect("The length of read bytes is somehow not 2"),
            )),
        }
    }
    pub fn read_i16(&mut self, endian: Endian) -> Result<i16, BufferReadError> {
        match endian {
            Endian::BigEndian => Ok(i16::from_be_bytes(
                self.read_bytes(2)?[0..2]
                    .try_into()
                    .expect("The length of read bytes is somehow not 2"),
            )),
            Endian::LittleEndian => Ok(i16::from_le_bytes(
                self.read_bytes(2)?[0..2]
                    .try_into()
                    .expect("The length of read bytes is somehow not 2"),
            )),
        }
    }
    pub fn skip(&mut self, amount: usize) -> Result<(), BufferReadError> {
        if self.index + amount > self.buffer.len() {
            return Err(BufferReadError::OutOfRange);
        }
        self.index += amount;
        return Ok(());
    }

    pub fn go_to(&mut self, offset: usize) -> Result<(), BufferReadError> {
        if offset > self.buffer.len() {
            return Err(BufferReadError::OutOfRange);
        }
        self.index = offset;
        return Ok(());
    }
}
