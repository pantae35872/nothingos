use alloc::{collections::BTreeMap, vec::Vec};

use crate::{graphics, utils::math::Polygon, BootInformation};

use super::ttf_parser::TtfParser;

pub struct TtfRenderer {
    data: Vec<char>,
    cache: BTreeMap<char, (Polygon, u32)>,
    parser: TtfParser<'static>,
    foreground_color: u32,
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
            data: Vec::new(),
            cache: BTreeMap::new(),
            parser,
            foreground_color,
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

    pub fn update(&mut self) {
        let mut offset = 1;
        let mut y_offset = 0;
        for charactor in &self.data {
            if *charactor == ' ' {
                offset += 15;
                if offset > 1800 {
                    y_offset += 1;
                    offset = 1;
                }
                continue;
            }

            if *charactor == '\n' {
                y_offset += 1;
                offset = 1;
                continue;
            }

            let (polygon, spaceing) = match self.cache.get(&charactor) {
                Some(polygon) => polygon,
                None => {
                    let mut polygon = self.parser.draw_char(&charactor);
                    polygon.0.scale(0.03);
                    polygon.0.set_y(100.0);
                    self.cache.insert(*charactor, polygon);
                    self.cache.get(charactor).unwrap()
                }
            };
            let mut polygon = polygon.clone();
            polygon.move_by((y_offset as f32 * 30.0) - 70.0);
            for pixel in polygon.render() {
                graphics::DRIVER.get().unwrap().lock().plot(
                    (pixel.x() + offset) as usize,
                    pixel.y() as usize,
                    self.foreground_color,
                );
            }
            offset += (*spaceing as i32 >> 5) + 0;
            if offset > 1800 {
                y_offset += 1;
                offset = 1;
            }
        }
    }
}
