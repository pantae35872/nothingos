use core::{
    f64::consts::PI,
    fmt::{write, Display},
};

use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub struct Coordinate {
    x: i32,
    y: i32,
}

impl Coordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn get_x(&self) -> i32 {
        return self.x;
    }

    pub fn get_y(&self) -> i32 {
        return self.y;
    }
}

fn sin(x: f64) -> f64 {
    let mut x = x;
    let mut sign = 1.0;
    if x < 0.0 {
        sign = -1.0;
        x = -x;
    }

    if x > 360.0 {
        x -= ((x / 360.0) as i32 * 360 as i32) as f64;
    }
    x *= PI / 180.0;

    let mut res = 0.0;
    let mut term = x;
    let mut k = 1.0;
    while res + term != res {
        res += term;
        k += 2.0;
        term *= -x * x / k / (k - 1.0);
    }

    return sign * res;
}

fn cos(x: f64) -> f64 {
    let mut x = x;
    if x < 0.0 {
        x = -x;
    }
    if x > 360.0 {
        x -= ((x / 360.0) as i32 * 360.0 as i32) as f64;
    }

    x *= PI / 180.0;
    let mut res = 0.0;
    let mut term = 1.0;
    let mut k = 0.0;
    while res + term != res {
        res += term;
        k += 2.0;
        term *= -x * x / k / (k - 1.0);
    }
    return res;
}

#[derive(Debug, Clone)]
pub struct Polygon {
    vertices: Vec<Vec<Coordinate>>,
}

impl Polygon {
    pub fn new(vertices: Vec<Vec<Coordinate>>) -> Self {
        Polygon { vertices }
    }

    pub fn vertices(&self) -> &Vec<Vec<Coordinate>> {
        &self.vertices
    }

    pub fn rotate(&mut self, angle: f64) {
        let cos_a = cos(angle);
        let sin_a = sin(angle);

        for ring in self.vertices.iter_mut() {
            for vertex in ring.iter_mut() {
                let x = vertex.x as f64;
                let y = vertex.y as f64;

                let new_x = x * cos_a - y * sin_a;
                let new_y = x * sin_a + y * cos_a;

                vertex.x = round(new_x) as i32;
                vertex.y = round(new_y) as i32;
            }
        }
    }

    pub fn flip(&mut self) {
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for ring in self.vertices.iter() {
            for vertex in ring.iter() {
                if vertex.y < min_y {
                    min_y = vertex.y;
                }
                if vertex.y > max_y {
                    max_y = vertex.y;
                }
            }
        }

        let mid_y = (max_y + min_y) / 2;

        for ring in self.vertices.iter_mut() {
            for vertex in ring.iter_mut() {
                vertex.y = mid_y - (vertex.y - mid_y);
            }
        }
    }

    pub fn move_down(&mut self, amount: i32) {
        for ring in self.vertices.iter_mut() {
            for vertex in ring.iter_mut() {
                vertex.y += amount;
            }
        }
    }

    pub fn scale(&mut self, scale_factor: f64) {
        for ring in self.vertices.iter_mut() {
            for vertex in ring.iter_mut() {
                vertex.x = round(vertex.x as f64 * scale_factor) as i32;
                vertex.y = round(vertex.y as f64 * scale_factor) as i32;
            }
        }
    }
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "X: {}, Y: {}", self.x, self.y)
    }
}

pub fn sqrt_approximate(n: i32) -> i32 {
    let mut x = n;
    let mut y = 1;
    while x > y {
        x = (x + y) / 2;
        y = n / x;
    }
    x
}

pub fn round(x: f64) -> i32 {
    if x >= 0.0 {
        (x + 0.5) as i32
    } else {
        (x - 0.5) as i32
    }
}
