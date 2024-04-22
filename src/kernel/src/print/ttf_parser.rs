use core::cmp::{max, min};

use crate::{
    inline_if, println, serial_println,
    utils::{
        buffer_reader::{BufferReader, Endian},
        math::Coordinate,
    },
};
use alloc::{string::String, vec, vec::Vec};

pub struct TtfParser<'a> {
    reader: BufferReader<'a>,
}

#[derive(Clone)]
pub struct GlyphFlag(u8);
impl GlyphFlag {
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn is_repeat(&self) -> bool {
        return ((self.0 >> 3) & 1) == 1;
    }
    pub fn is_on_curve(&self) -> bool {
        return ((self.0 >> 0) & 1) == 1;
    }

    pub fn is_set(&self, offset: usize) -> bool {
        return ((self.0 >> offset) & 1) == 1;
    }
}

#[derive(Debug)]
struct GlyphData {
    coords: Vec<Coordinate>,
    contour_end_indices: Vec<u16>,
}

impl<'a> TtfParser<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            reader: BufferReader::new(buffer),
        }
    }

    pub fn test(&mut self) {
        self.reader.skip(4).expect("not skip byte in ttf");
        let num_table = self
            .reader
            .read_u16(Endian::BigEndian)
            .expect("Cannot read numtable");
        self.reader.skip(6).unwrap();
        serial_println!("numtable: {}", num_table);
        for _ in 0..num_table {
            let tag = self.read_tag();
            let checksum = self.reader.read_u32(Endian::BigEndian).unwrap();
            let offset = self.reader.read_u32(Endian::BigEndian).unwrap();
            let length = self.reader.read_u32(Endian::BigEndian).unwrap();
            serial_println!("Tag: {} Location: {} Checksum: {}", tag, offset, checksum);
            if tag == "glyf" {
                self.reader.go_to(offset as usize).unwrap();
                serial_println!("{:?}", self.read_glyph().unwrap());
                break;
            }
        }
    }

    fn read_glyph(&mut self) -> Option<GlyphData> {
        let contour_end_indices_count = self.reader.read_i16(Endian::BigEndian).unwrap();
        if contour_end_indices_count < 0 {
            serial_println!("Glyph is compound");
            return None;
        }
        self.reader.skip(8).unwrap();
        let mut contour_end_indices: Vec<u16> = vec![0u16; contour_end_indices_count as usize];
        for i in 0..contour_end_indices_count {
            contour_end_indices[i as usize] = self.reader.read_u16(Endian::BigEndian).unwrap();
        }

        let num_points = contour_end_indices[contour_end_indices.len() - 1] + 1;
        let mut allflags: Vec<GlyphFlag> = vec![GlyphFlag::new(0u8); num_points.into()];
        let instruction_size = self.reader.read_i16(Endian::BigEndian).unwrap() as usize;
        self.reader.skip(instruction_size).unwrap();
        for i in 0..num_points {
            let flag = GlyphFlag::new(self.reader.read_byte().unwrap());
            allflags[i as usize] = flag.clone();

            if flag.is_repeat() {
                for r in 0..self.reader.read_byte().unwrap() {
                    allflags[r as usize + 1] = flag.clone();
                }
            }
        }

        let coords_x = self.read_coordinates(&allflags, true);
        let coords_y = self.read_coordinates(&allflags, false);
        let mut coords = Vec::new();
        for i in 0..min(coords_x.len(), coords_y.len()) {
            coords.push(Coordinate::new(coords_x[i], coords_y[i]));
        }

        return Some(GlyphData {
            coords,
            contour_end_indices,
        });
    }

    pub fn read_coordinates(&mut self, all_flags: &Vec<GlyphFlag>, is_read_x: bool) -> Vec<i32> {
        let offset_size_flag_bit = inline_if!(is_read_x, 1, 2);
        let offset_sige_or_skip_bit = inline_if!(is_read_x, 4, 5);
        let mut coordinates = vec![0i32; all_flags.len()];
        for i in 0..coordinates.len() {
            coordinates[i] = coordinates[max(0, i as i32 - 1) as usize];
            let flag = &all_flags[i];
            let on_curve = flag.is_on_curve();
            if flag.is_set(offset_size_flag_bit) {
                let offset = self.reader.read_byte().unwrap() as i32;
                let sign: i32 = inline_if!(flag.is_set(offset_sige_or_skip_bit), 1, -1);
                coordinates[i] += offset * sign;
            } else if !flag.is_set(offset_sige_or_skip_bit) {
                coordinates[i] += self.reader.read_i16(Endian::BigEndian).unwrap() as i32;
            }
        }
        return coordinates;
    }

    pub fn read_tag(&mut self) -> String {
        String::from_utf8(
            self.reader
                .read_bytes(4)
                .expect("Could not read tag")
                .to_vec(),
        )
        .expect("Could not format tag")
    }
}
