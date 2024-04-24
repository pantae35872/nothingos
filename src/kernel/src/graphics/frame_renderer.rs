use alloc::vec;
use alloc::vec::Vec;

use super::Coordinate;

pub struct FrameRenderer {
    pixels: Vec<Vec<u32>>,
    width: usize,
    height: usize,
}

impl FrameRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![vec![0u32; width]; height],
            width,
            height,
        }
    }

    pub fn put_pixel(&mut self, coordinate: Coordinate, color: u32) {
        self.pixels[coordinate.x()][coordinate.y()] = color;
    }

    pub fn render(self) -> Vec<(Coordinate, u32)> {
        let mut value = Vec::new();
        for (i, x) in self.pixels.iter().enumerate() {
            for (y, color) in x.iter().enumerate() {
                value.push((Coordinate::new(i, y), *color));
            }
        }
        return value;
    }
}
