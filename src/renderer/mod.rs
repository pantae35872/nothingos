use alloc::vec;
use alloc::vec::Vec;

pub struct FrameRenderer {
    pixels: Vec<Vec<u8>>,
    width: usize,
    height: usize,
}

impl FrameRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![vec![0; height]; width],
            width,
            height,
        }
    }
}

impl Renderer for FrameRenderer {
    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        self.pixels[x][y] = color;
    }

    fn get_at_pos(&self, x: usize, y: usize) -> u8 {
        self.pixels[x][y]
    }
    fn get_width(&self) -> usize {
        self.width
    }
    fn get_height(&self) -> usize {
        self.height
    }
}

pub trait Renderer {
    fn set_pixel(&mut self, x: usize, y: usize, color: u8);
    fn get_at_pos(&self, x: usize, y: usize) -> u8;
    fn get_height(&self) -> usize;
    fn get_width(&self) -> usize;
}
