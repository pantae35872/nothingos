use core::slice::{self, from_raw_parts_mut};

use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use spin::mutex::Mutex;
use uefi::proto::console::gop::{Mode, ModeInfo, PixelFormat};

use crate::{serial_println, utils::math::Vector2, BootInformation};

pub static DRIVER: OnceCell<Mutex<Graphic>> = OnceCell::uninit();

pub struct Graphic {
    mode: ModeInfo,
    frame_buffer: &'static mut [u32],
}

impl Graphic {
    pub fn new(mode: ModeInfo, frame_buffer: &'static mut [u32]) -> Self {
        Self { mode, frame_buffer }
    }

    pub fn plot(&mut self, x: usize, y: usize, color: u32) {
        let (width, height) = self.mode.resolution();
        if x > width || y > height {
            return;
        }

        match self.mode.pixel_format() {
            PixelFormat::Rgb => {
                self.frame_buffer[y * width + x] = color << 8;
            }
            PixelFormat::Bgr => {
                self.frame_buffer[y * width + x] = color;
            }
            PixelFormat::Bitmask => {}
            PixelFormat::BltOnly => {}
        }
    }
}

pub fn init(bootinfo: &BootInformation) {
    DRIVER.init_once(|| unsafe {
        let (width, height) = bootinfo.gop_mode.info().resolution();
        let buffer = from_raw_parts_mut(bootinfo.framebuffer, width * height * 4);
        Mutex::new(Graphic::new(bootinfo.gop_mode.info().clone(), buffer))
    });
}

#[derive(Debug, Clone, Copy)]
pub struct Coordinate {
    x: i32,
    y: i32,
}

impl Coordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> i32 {
        return self.x;
    }

    pub fn y(&self) -> i32 {
        return self.y;
    }
}

pub fn draw_line(start: &Vector2, end: &Vector2, points: &mut Vec<Vector2>) {
    let start = start.as_coordinate();
    let end = end.as_coordinate();

    let dx = ((end.x() - start.x()) as i32).abs();
    let dy = ((end.y() - start.y()) as i32).abs();
    let sx = if start.x() < end.x() { 1 } else { -1 };
    let sy = if start.y() < end.y() { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = start.x() as i32;
    let mut y = start.y() as i32;

    loop {
        points.push(Vector2::new(x as f32, y as f32));

        if x == end.x() as i32 && y == end.y() as i32 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn linear_interpolation(start: Vector2, end: Vector2, t: f32) -> Vector2 {
    return start + (end - start) * t;
}

pub fn bezier_interpolation(p0: Vector2, p1: Vector2, p2: Vector2, t: f32) -> Vector2 {
    let intermediate_a = linear_interpolation(p0, p1, t);
    let intermediate_b = linear_interpolation(p1, p2, t);
    return linear_interpolation(intermediate_a, intermediate_b, t);
}

pub fn draw_bezier(p0: Vector2, p1: Vector2, p2: Vector2, res: i32, points: &mut Vec<Vector2>) {
    let mut prev_point_on_curve = p0;
    for i in 0..res {
        let t = (i as f32 + 1.0) / res as f32;
        let next_point_on_curve = bezier_interpolation(p0, p1, p2, t);
        draw_line(&prev_point_on_curve, &next_point_on_curve, points);
        prev_point_on_curve = next_point_on_curve;
    }
}
