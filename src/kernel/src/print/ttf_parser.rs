use core::{
    char,
    cmp::{max, min},
};

use crate::{
    graphics::{draw_bezier, draw_line},
    inline_if, println, serial_println,
    utils::{
        buffer_reader::{BufferReader, Endian},
        math::{Polygon, Vector2},
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
    glyph: Vec<u32>,
    mappings: BTreeMap<u32, usize>,
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
pub enum ParseTtfError {
    FormatNotSupported,
}

#[derive(Debug)]
pub struct GlyphData {
    pub contours: Vec<(bool, Vector2)>,
    pub contour_end_indices: Vec<u16>,
}

impl<'a> TtfParser<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            reader: BufferReader::new(buffer),
            tables: BTreeMap::new(),
            glyph: Vec::new(),
            mappings: BTreeMap::new(),
        }
    }

    pub fn parse(&mut self) -> Result<(), ParseTtfError> {
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
        self.glyph = self.get_all_glyph_locations();
        let cmap = self.tables.get("cmap").unwrap();
        self.reader.go_to(cmap.offset as usize).unwrap();

        let version = self.reader.read_u16(Endian::BigEndian).unwrap();
        let num_sub_tables = self.reader.read_u16(Endian::BigEndian).unwrap();

        let mut cmap_subtable_offset = u32::MAX;

        for i in 0..num_sub_tables {
            let platform_id = self.reader.read_u16(Endian::BigEndian).unwrap();
            let platform_specific_id = self.reader.read_u16(Endian::BigEndian).unwrap();
            let offset = self.reader.read_u32(Endian::BigEndian).unwrap();

            if platform_id == 0 {
                let unicode_version_info = platform_specific_id;

                if unicode_version_info == 4 {
                    cmap_subtable_offset = offset;
                }

                if unicode_version_info == 3 && cmap_subtable_offset == u32::MAX {
                    cmap_subtable_offset = offset;
                }
            }
        }

        if cmap_subtable_offset == 0 {
            return Err(ParseTtfError::FormatNotSupported);
        }

        self.reader
            .go_to(cmap.offset as usize + cmap_subtable_offset as usize)
            .unwrap();

        let format = self.reader.read_u16(Endian::BigEndian).unwrap();
        if format != 12 && format != 4 {
            return Err(ParseTtfError::FormatNotSupported);
        } else if format == 12 {
            let _reserved = self.reader.read_u16(Endian::BigEndian).unwrap();
            let subtable_byte_length_including_header =
                self.reader.read_u32(Endian::BigEndian).unwrap();
            let language_code = self.reader.read_u32(Endian::BigEndian).unwrap();
            let num_groups = self.reader.read_u32(Endian::BigEndian).unwrap();

            for i in 0..num_groups {
                let start_char_code = self.reader.read_u32(Endian::BigEndian).unwrap();
                let end_char_code = self.reader.read_u32(Endian::BigEndian).unwrap();
                let start_glyph_index = self.reader.read_u32(Endian::BigEndian).unwrap();

                let num_chars = end_char_code - start_char_code + 1;
                for char_code_offset in 0..num_chars {
                    let char_code = start_char_code + char_code_offset;
                    let glyph_index = start_glyph_index + char_code_offset;

                    self.mappings.insert(char_code, glyph_index as usize);
                }
            }
        } else if format == 4 {
            self.reader.skip(4).unwrap();

            let seg_count_2x = self.reader.read_u16(Endian::BigEndian).unwrap();
            let seg_count = seg_count_2x / 2;
            self.reader.skip(6).unwrap();
            let mut end_codes = vec![0i32; seg_count.into()];
            for end_code in end_codes.iter_mut() {
                *end_code = self.reader.read_u16(Endian::BigEndian).unwrap().into();
            }

            self.reader.skip(2).unwrap();
            let mut start_codes = vec![0i32; seg_count.into()];
            for start_code in start_codes.iter_mut() {
                *start_code = self.reader.read_u16(Endian::BigEndian).unwrap().into();
            }

            let mut id_deltas = vec![0i32; seg_count.into()];
            for id_delta in id_deltas.iter_mut() {
                *id_delta = self.reader.read_u16(Endian::BigEndian).unwrap().into();
            }

            let mut id_range_offsets = vec![(0i32, 0i32); seg_count.into()];
            for id_range_offset in id_range_offsets.iter_mut() {
                let read_loc = self.reader.get_index() as i32;
                let offset = self.reader.read_u16(Endian::BigEndian).unwrap();
                *id_range_offset = (offset.into(), read_loc);
            }

            for i in 0..start_codes.len() {
                let end_code = end_codes[i];
                let mut curr_code = start_codes[i];

                while curr_code <= end_code {
                    let mut glyph_index = 0;

                    if id_range_offsets[i].0 == 0 {
                        glyph_index = (curr_code + id_deltas[i]) % 65536;
                    } else {
                        let range_offset_location = id_range_offsets[i].1 + id_range_offsets[i].0;
                        let glyph_index_array_location =
                            2 * (curr_code - start_codes[i]) + range_offset_location;

                        let reader_location_old = self.reader.get_index();
                        self.reader
                            .go_to(glyph_index_array_location as usize)
                            .unwrap();
                        let glyph_index_offset = self.reader.read_u16(Endian::BigEndian).unwrap();
                        self.reader.go_to(reader_location_old).unwrap();

                        if glyph_index_offset != 0 {
                            glyph_index = (glyph_index_offset + id_deltas[i] as u16) as i32 % 65536;
                        }
                    }

                    self.mappings.insert(curr_code as u32, glyph_index as usize);
                    curr_code += 1;
                }
            }
        }
        return Ok(());
    }

    pub fn draw_char(&mut self, charactor: &char) -> Polygon {
        let mut buffer = [0; 4];
        charactor.encode_utf8(&mut buffer);
        let index = self.mappings.get(&u32::from_le_bytes(buffer)).unwrap();
        if let Some(glyph) = self.read_glyph(self.glyph[*index] as usize) {
            return Self::render(&glyph);
        } else {
            let glyph = self.read_glyph(self.glyph[0] as usize).unwrap();
            return Self::render(&glyph);
        }
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
        let mut data = Vec::new();
        let contours = Self::implied_points(glyph);

        for points in contours {
            for i in (0..points.len()).step_by(2) {
                let p0 = points[i];
                let p1 = points[(i + 1) % points.len()];
                let p2 = points[(i + 2) % points.len()];
                draw_bezier(p0, p1, p2, 100, &mut data);
            }
        }

        return Polygon::new(data);
    }

    fn implied_points(glyph: &GlyphData) -> Vec<Vec<Vector2>> {
        let mut contours = Vec::new();
        let mut contour_start = 0;
        for contour_end in &glyph.contour_end_indices {
            let original_contour = &glyph.contours[(contour_start as usize)
                ..(contour_start as usize + (contour_end - contour_start + 1) as usize)];
            let mut point_offset = 0;
            while point_offset < original_contour.len() {
                if original_contour[point_offset].0 {
                    break;
                }
                point_offset += 1;
            }

            let mut new_contour = Vec::new();
            for i in 0..=original_contour.len() {
                let curr = original_contour[(i + point_offset) % original_contour.len()];
                let next = original_contour[(i + point_offset + 1) % original_contour.len()];
                new_contour.push(Vector2::new(curr.1.x(), curr.1.y()));

                if curr.0 == next.0 && i < original_contour.len() {
                    let midpoint =
                        Vector2::new(curr.1.x() + next.1.x(), curr.1.y() + next.1.y()) / 2.0;
                    new_contour.push(midpoint);
                }
            }
            contours.push(new_contour);
            contour_start = contour_end + 1;
        }

        return contours;
    }

    fn read_compound_glyph(&mut self, glyph_location: usize) -> GlyphData {
        self.reader.go_to(glyph_location as usize).unwrap();
        self.reader.skip(2 * 5).unwrap();

        let mut points: Vec<(bool, Vector2)> = Vec::new();
        let mut contour_end_indices = Vec::new();

        loop {
            let (mut compoent_glyph, is_last) = self.read_next_component_glyph();

            let index_offset = points.len();
            points.append(&mut compoent_glyph.contours);

            for end_index in compoent_glyph.contour_end_indices {
                contour_end_indices.push(end_index + index_offset as u16);
            }

            if is_last {
                break;
            }
        }
        return GlyphData {
            contours: points,
            contour_end_indices,
        };
    }

    fn read_next_component_glyph(&mut self) -> (GlyphData, bool) {
        let flags = self.reader.read_u16(Endian::BigEndian).unwrap();
        let glyph_index = self.reader.read_u16(Endian::BigEndian).unwrap();
        let previous_location = self.reader.get_index();
        let mut glyph = self
            .read_glyph(self.glyph[glyph_index as usize] as usize)
            .unwrap();
        self.reader.go_to(previous_location).unwrap();

        let offset_x = inline_if!(
            ((flags >> 0) & 1) == 1,
            self.reader.read_i16(Endian::BigEndian).unwrap() as f32,
            self.reader.read_byte().unwrap() as f32
        );
        let offset_y = inline_if!(
            ((flags >> 0) & 1) == 1,
            self.reader.read_i16(Endian::BigEndian).unwrap() as f32,
            self.reader.read_byte().unwrap() as f32
        );

        if (flags >> 3) & 1 == 1 {
            self.reader.skip(6).unwrap();
        } else if (flags >> 6) & 1 == 1 {
            self.reader.skip(6 * 2).unwrap();
        }

        for point in glyph.contours.iter_mut() {
            point.1 = point.1 + Vector2::new(offset_x, offset_y);
        }

        return (glyph, (flags >> 5) & 1 != 1);
    }

    fn read_glyph(&mut self, glyph_location: usize) -> Option<GlyphData> {
        self.reader.go_to(glyph_location).unwrap();
        let contour_end_indices_count = self.reader.read_i16(Endian::BigEndian).unwrap();
        if contour_end_indices_count < 0 {
            return Some(self.read_compound_glyph(glyph_location));
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
        let mut contours = Vec::new();
        for i in 0..min(coords_x.len(), coords_y.len()) {
            contours.push((
                coords_x[i].0 | coords_y[i].0,
                Vector2::new(coords_x[i].1 as f32, coords_y[i].1 as f32),
            ));
        }

        return Some(GlyphData {
            contours,
            contour_end_indices,
        });
    }

    pub fn read_coordinates(
        &mut self,
        all_flags: &Vec<GlyphFlag>,
        is_read_x: bool,
    ) -> Vec<(bool, i32)> {
        let offset_size_flag_bit = inline_if!(is_read_x, 1, 2);
        let offset_sige_or_skip_bit = inline_if!(is_read_x, 4, 5);
        let mut coordinates = vec![(false, 0i32); all_flags.len()];
        for i in 0..coordinates.len() {
            coordinates[i] = coordinates[max(0, i as i32 - 1) as usize];
            let flag = &all_flags[i];
            let on_curve = flag.is_on_curve();
            coordinates[i].0 = on_curve;
            if flag.is_set(offset_size_flag_bit) {
                let offset = self.reader.read_byte().unwrap() as i32;
                let sign: i32 = inline_if!(flag.is_set(offset_sige_or_skip_bit), 1, -1);
                coordinates[i].1 += offset * sign;
            } else if !flag.is_set(offset_sige_or_skip_bit) {
                coordinates[i].1 += self.reader.read_i16(Endian::BigEndian).unwrap() as i32;
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
