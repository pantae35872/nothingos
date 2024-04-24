use core::{
    cmp::Ordering,
    f64::consts::PI,
    fmt::{write, Display},
    ops::{Add, Div, Mul, Sub},
};

use alloc::{collections::BinaryHeap, vec::Vec};

use crate::graphics::Coordinate;

struct Edge {
    y_upper: f32,
    x_intercept: f32,
    slope_inverse: f32,
}

impl Edge {
    fn new(p1: Vector2, p2: Vector2) -> Self {
        let (lower, upper) = if p1.y < p2.y { (p1, p2) } else { (p2, p1) };
        Edge {
            y_upper: upper.y,
            x_intercept: lower.x,
            slope_inverse: (upper.x - lower.x) / (upper.y - lower.y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn render(&self) -> Vec<Coordinate> {
        return self
            .data
            .iter()
            .map(|e| Coordinate::new(e.x() as i32, e.y() as i32))
            .collect();
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

    pub fn move_down(&mut self, amount: f32) {
        for vertex in self.data.iter_mut() {
            vertex.y += amount;
        }
    }

    pub fn fill(&mut self) {
        let mut edges: Vec<Edge> = Vec::new();
        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let polygon = &self.data;
        for i in 0..polygon.len() {
            y_min = f32::min(y_min, polygon[i].y);
            y_max = f32::max(y_max, polygon[i].y);
            let next_index = if i == polygon.len() - 1 { 0 } else { i + 1 };
            let edge = Edge::new(polygon[i], polygon[next_index]);
            edges.push(edge);
        }

        let mut scanline: Vec<Vector2> = Vec::new();
        let mut active_edges: BinaryHeap<(i32, i32)> = BinaryHeap::new(); // (x, dx)

        for y in (y_min as i32)..=(y_max as i32) {
            edges.retain(|edge| edge.y_upper > y as f32);
            for edge in &edges {
                if edge.y_upper > y as f32 {
                    active_edges.push((
                        (edge.x_intercept * 10.0) as i32,
                        (edge.slope_inverse * 10.0) as i32,
                    ));
                }
            }

            // Fill pixels between active edges
            let mut x_min = f32::INFINITY;
            let mut x_max = f32::NEG_INFINITY;
            while let Some((x, dx)) = active_edges.pop() {
                let x_f32 = x as f32 / 10.0;
                x_min = f32::min(x_min, x_f32);
                x_max = f32::max(x_max, x_f32);

                if let Some((nx, _)) = active_edges.peek() {
                    if *nx != x {
                        for xx in (x_min as i32)..=(x_max as i32) {
                            scanline.push(Vector2::new(xx as f32, y as f32));
                        }
                        x_min = f32::INFINITY;
                        x_max = f32::NEG_INFINITY;
                    }
                } else {
                    for xx in (x_min as i32)..=(x_max as i32) {
                        scanline.push(Vector2::new(xx as f32, y as f32));
                    }
                }

                if dx > 0 {
                    active_edges.push((x + dx, dx));
                }
            }
        }
        self.data = scanline;
    }

    pub fn scale(&mut self, scale_factor: f64) {
        for vertex in self.data.iter_mut() {
            vertex.x = round(vertex.x() as f64 * scale_factor) as f32;
            vertex.y = round(vertex.y() as f64 * scale_factor) as f32;
        }
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
