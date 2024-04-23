use alloc::vec::Vec;

use crate::{serial_println, utils::math::Coordinate};

pub fn draw_line(start: &Coordinate, end: &Coordinate) -> Vec<Coordinate> {
    let mut result = Vec::new();

    let dx = (end.get_x() - start.get_x()).abs();
    let dy = (end.get_y() - start.get_y()).abs();
    let sx = if start.get_x() < end.get_x() { 1 } else { -1 };
    let sy = if start.get_y() < end.get_y() { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = start.get_x();
    let mut y = start.get_y();

    loop {
        result.push(Coordinate::new(x, y));

        if x == end.get_x() && y == end.get_y() {
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

    result
}

pub fn draw_triangle(
    point1: &Coordinate,
    point2: &Coordinate,
    point3: &Coordinate,
) -> Vec<Vec<Coordinate>> {
    let mut lines = Vec::new();
    lines.push(draw_line(point1, point2));
    lines.push(draw_line(point2, point3));
    lines.push(draw_line(point3, point1));
    lines
}
