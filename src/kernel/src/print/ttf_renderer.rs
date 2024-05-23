use alloc::{collections::BTreeMap, vec::Vec};

use crate::{graphics, utils::math::Polygon, BootInformation};

use super::ttf_parser::TtfParser;

pub struct TtfRenderer {
    data: Vec<char>,
    cache: BTreeMap<char, (Polygon, u32)>,
    parser: TtfParser<'static>,
    foreground_color: u32,
    curr_line: u64,
}

impl TtfRenderer {
    pub fn new(boot_info: &BootInformation, foreground_color: u32) -> Self {
        let mut parser = unsafe {
            TtfParser::new(core::slice::from_raw_parts_mut(
                boot_info.font_start as *mut u8,
                (boot_info.font_end - boot_info.font_start) as usize,
            ))
        };
        parser.parse().unwrap();
        Self {
            data: Vec::with_capacity(5000),
            cache: BTreeMap::new(),
            parser,
            foreground_color,
            curr_line: 0,
        }
    }

    pub fn set_color(&mut self, color: &u32) {
        self.foreground_color = *color;
    }

    pub fn put_char(&mut self, charactor: &char) {
        self.data.push(*charactor);
    }

    pub fn put_str(&mut self, string: &str) {
        for char in string.chars() {
            self.put_char(&char);
        }
        self.update();
    }

    pub fn cache(&mut self, charactor: &char) -> bool {
        match self.cache.get_mut(&charactor) {
            Some(_) => {
                return true;
            }
            None => {
                let mut polygon = self.parser.draw_char(&charactor);
                polygon.0.set_y(100.0);
                self.cache.insert(*charactor, polygon);
                return false;
            }
        };
    }

    pub fn update(&mut self) {
        let mut offset = 1;
        let mut y_offset = 0;
        let mut graphics = graphics::DRIVER.get().unwrap().lock();
        let (horizontal, _vertical) = graphics.get_res();
        for charactor in &self.data {
            if *charactor == ' ' {
                offset += 16;
                if offset > horizontal as i32 {
                    y_offset += 1;
                    offset = 16;
                }
                continue;
            }

            if *charactor == '\n' {
                y_offset += 1;
                offset = 1;
                continue;
            }

            let (polygon, spaceing) = match self.cache.get_mut(&charactor) {
                Some(polygon) => polygon,
                None => {
                    let mut polygon = self.parser.draw_char(&charactor);
                    polygon.0.set_y(100.0);
                    self.cache.insert(*charactor, polygon);
                    self.cache.get_mut(charactor).unwrap()
                }
            };

            if y_offset >= self.curr_line {
                polygon.move_by((y_offset as f32 * 20.0) - 80.0);
                for pixel in polygon.render() {
                    graphics.plot(
                        (pixel.x() as i32 + offset) as usize,
                        pixel.y() as usize,
                        self.foreground_color,
                    );
                }
                polygon.move_by(-((y_offset as f32 * 20.0) - 80.0));
            }
            offset += (*spaceing as i32 >> 6) + 5;
            if offset > horizontal as i32 {
                y_offset += 1;
                offset = 0;
            }
        }
        self.curr_line = y_offset;
    }
}
