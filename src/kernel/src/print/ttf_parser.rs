use core::cmp::{max, min};

use crate::{
    graphics::draw_line,
    inline_if, println, serial_println,
    utils::{
        buffer_reader::{BufferReader, Endian},
        math::{Coordinate, Polygon},
    },
};
use alloc::{
    collections::{btree_map, BTreeMap},
    string::String,
    vec,
    vec::Vec,
};

pub struct TtfParser<'a> {
    reader: BufferReader<'a>,
    tables: BTreeMap<String, TableHeader>,
}

struct TableHeader {
    checksum: u32,
    offset: u32,
    length: u32,
}

impl TableHeader {
    pub fn new(checksum: u32, offset: u32, length: u32) -> Self {
        Self {
            checksum,
            offset,
            length,
        }
    }
}

#[derive(Clone, Copy)]
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
pub struct GlyphData {
    pub coords: Vec<Coordinate>,
    pub contour_end_indices: Vec<u16>,
}

impl<'a> TtfParser<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            reader: BufferReader::new(buffer),
            tables: BTreeMap::new(),
        }
    }

    pub fn test(&mut self) -> Vec<Polygon> {
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
            self.tables
                .insert(tag, TableHeader::new(checksum, offset, length));
        }
        let mut polygons = Vec::new();
        let mut i = 0;
        for location in self.get_all_glyph_locations() {
            self.reader.go_to(location as usize).unwrap();
            if let Some(glyph) = self.read_glyph() {
                if i > 50 {
                    break;
                }
                polygons.push(Self::render(&glyph));
                i += 1;
            };
        }
        return polygons;
    }

    fn get_all_glyph_locations(&mut self) -> Vec<u32> {
        self.reader
            .go_to((self.tables.get("maxp").unwrap().offset + 4) as usize)
            .unwrap();
        let num_glyphs = self.reader.read_u16(Endian::BigEndian).unwrap();
        self.reader
            .go_to(self.tables.get("head").unwrap().offset as usize)
            .unwrap();
        self.reader.skip(50).unwrap();
        let is_two_byte_entry = self.reader.read_i16(Endian::BigEndian).unwrap() == 0;

        let location_table_start = self.tables.get("loca").unwrap().offset;
        let glyph_table_start = self.tables.get("glyf").unwrap().offset;
        let mut all_glyph_locations = vec![0u32; num_glyphs as usize];
        for glyph_index in 0..num_glyphs {
            self.reader
                .go_to(
                    location_table_start as usize
                        + glyph_index as usize * (inline_if!(is_two_byte_entry, 2, 4)),
                )
                .unwrap();
            let glyph_data_offset = inline_if!(
                is_two_byte_entry,
                self.reader.read_u16(Endian::BigEndian).unwrap() as u32 * 2,
                self.reader.read_u32(Endian::BigEndian).unwrap()
            );
            all_glyph_locations[glyph_index as usize] = glyph_table_start + glyph_data_offset;
        }

        return all_glyph_locations;
    }

    fn render(glyph: &GlyphData) -> Polygon {
        let mut contour_start_index = 0;
        let mut lines = Vec::new();
        for contour_end_index in &glyph.contour_end_indices {
            let num_points_in_contour = contour_end_index - contour_start_index + 1;
            let points = &glyph.coords[(contour_start_index as usize)
                ..((contour_start_index + num_points_in_contour) as usize)];
            for i in 0..points.len() {
                lines.push(draw_line(&points[i], &points[(i + 1) % points.len()]));
            }

            contour_start_index = contour_end_index + 1;
        }
        return Polygon::new(lines);
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
        let mut i = 0;
        while i < num_points {
            let flag = GlyphFlag::new(self.reader.read_byte().unwrap());
            allflags[i as usize] = flag;

            if flag.is_repeat() {
                for _ in 0..self.reader.read_byte().unwrap() {
                    i += 1;
                    allflags[i as usize] = flag;
                }
            }
            i += 1;
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
