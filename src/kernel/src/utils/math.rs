use core::{
    f64::consts::PI,
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
};

use alloc::vec::Vec;

use crate::graphics::{draw_line, Coordinate};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vector2 {
    x: f32,
    y: f32,
}
impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> f32 {
        return self.x;
    }

    pub fn set_x(&mut self, value: f32) {
        self.x = value;
    }

    pub fn set_y(&mut self, value: f32) {
        self.y = value;
    }

    pub fn y(&self) -> f32 {
        return self.y;
    }

    pub fn as_coordinate(&self) -> Coordinate {
        return Coordinate::new(self.x as i32, self.y as i32);
    }
}
impl Sub for Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Div<f32> for Vector2 {
    type Output = Vector2;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x() / rhs,
            y: self.y() / rhs,
        }
    }
}

impl Add for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
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
    data: Vec<Vector2>,
}

impl Polygon {
    pub fn new(data: Vec<Vector2>) -> Self {
        Polygon { data }
    }

    pub fn render(&self) -> &Vec<Vector2> {
        return &self.data;
    }

    pub fn flip(&mut self) {
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for vertex in self.data.iter() {
            if vertex.y() < min_y {
                min_y = vertex.y;
            }
            if vertex.y() > max_y {
                max_y = vertex.y;
            }
        }

        let mid_y = (max_y + min_y) / 2.0;

        for vertex in self.data.iter_mut() {
            vertex.y = mid_y - (vertex.y - mid_y);
        }
    }

    pub fn move_by(&mut self, amount: f32) {
        for vertex in self.data.iter_mut() {
            vertex.y += amount;
        }
    }

    pub fn set_y(&mut self, location: f32) {
        let maxy = self
            .data
            .iter()
            .max_by(|x, y| x.y().partial_cmp(&y.y()).unwrap())
            .unwrap()
            .y as i32;
        let miny = self
            .data
            .iter()
            .min_by(|x, y| x.y().partial_cmp(&y.y()).unwrap())
            .unwrap()
            .y as i32;

        for (c, i) in (miny.max(0)..=maxy).enumerate() {
            for line in self.data.iter_mut().filter(|e| e.y() as i32 == i) {
                line.y = location - c as f32;
            }
        }
        for (c, i) in (miny.min(0)..0).rev().enumerate() {
            for line in self.data.iter_mut().filter(|e| e.y() as i32 == i) {
                line.y = location + c as f32;
            }
        }
    }

    pub fn fill(&mut self) {
        let data = self.data.clone();
        let maxy = self
            .data
            .iter()
            .max_by(|x, y| x.y().partial_cmp(&y.y()).unwrap())
            .unwrap();
        let miny = self
            .data
            .iter()
            .min_by(|x, y| x.y().partial_cmp(&y.y()).unwrap())
            .unwrap();

        for i in (miny.y as i32)..(maxy.y as i32) {
            let mut line: Vec<&Vector2> = data.iter().filter(|e| e.y() as i32 == i).collect();
            line.sort_by(|x, y| x.x().partial_cmp(&y.x()).unwrap());
            if !line.is_empty() {
                let mut corners = Vec::new();
                line.dedup();
                for i in 0..line.len() {
                    let se = line.get(i).unwrap();

                    if i == 0 {
                        if !line.get(1).is_some_and(|e| e.x == se.x + 1.0) {
                            corners.push(se);
                        }
                        continue;
                    }

                    if i == line.len() - 1 {
                        if !line.get(i - 1).is_some_and(|e| e.x == se.x - 1.0) {
                            corners.push(se);
                        }
                        continue;
                    }

                    if !(line.get(i - 1).is_some_and(|e| e.x == se.x - 1.0)
                        && line.get(i + 1).is_some_and(|e| e.x == se.x + 1.0))
                    {
                        corners.push(se);
                    }
                }

                if corners.len() % 2 == 0 {
                    for pair in corners.chunks_exact(2) {
                        draw_line(pair[0], pair[1], &mut self.data);
                    }
                } else if corners.len() % 2 != 0 {
                    let middle_index = corners.len() / 2;
                    corners.remove(middle_index);
                    for pair in corners.chunks_exact(2) {
                        draw_line(pair[0], pair[1], &mut self.data);
                    }
                }
            }
        }
    }

    pub fn scale(&mut self, scale_factor: f64) {
        for vertex in self.data.iter_mut() {
            vertex.x = round(vertex.x() as f64 * scale_factor) as f32;
            vertex.y = round(vertex.y() as f64 * scale_factor) as f32;
        }
        self.data.dedup();
    }
}

impl Display for Vector2 {
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
