#![forbid(unsafe_code)]

use std::ops::{Add, Sub};

use crate::style::FlexDirection;

#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Rect {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.left + self.right,
            height: self.top + self.bottom,
        }
    }

    pub fn main(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.left + self.right
        } else {
            self.top + self.bottom
        }
    }

    pub fn main_start(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.left
        } else {
            self.top
        }
    }

    pub fn main_end(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.right
        } else {
            self.bottom
        }
    }

    pub fn cross(&self, dir: FlexDirection) -> f32 {
        if !dir.is_row() {
            self.left + self.right
        } else {
            self.top + self.bottom
        }
    }

    pub fn cross_start(&self, dir: FlexDirection) -> f32 {
        if !dir.is_row() {
            self.left
        } else {
            self.top
        }
    }

    pub fn cross_end(&self, dir: FlexDirection) -> f32 {
        if !dir.is_row() {
            self.right
        } else {
            self.bottom
        }
    }
}

impl Add for Rect {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            top: self.top + other.top,
            right: self.right + other.right,
            bottom: self.bottom + other.bottom,
            left: self.left + other.left,
        }
    }
}

impl Sub for Rect {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            top: self.top - other.top,
            right: self.right - other.right,
            bottom: self.bottom - other.bottom,
            left: self.left - other.left,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            width: self.width + other.width,
            height: self.height + other.height,
        }
    }
}

impl Sub for Size {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            width: self.width - other.width,
            height: self.height - other.height,
        }
    }
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self { width: 0.0, height: 0.0 }
    }

    pub fn infinite() -> Self {
        Self {
            width: f32::INFINITY,
            height: f32::INFINITY,
        }
    }

    pub fn is_infinite(&self) -> bool {
        self.width.is_infinite() && self.height.is_infinite()
    }

    pub fn main(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.width
        } else {
            self.height
        }
    }

    pub fn cross(&self, dir: FlexDirection) -> f32 {
        if !dir.is_row() {
            self.width
        } else {
            self.height
        }
    }

    pub fn set_main(&mut self, dir: FlexDirection, len: f32) {
        if dir.is_row() {
            self.width = len;
        } else {
            self.height = len;
        }
    }

    pub fn set_cross(&mut self, dir: FlexDirection, len: f32) {
        if !dir.is_row() {
            self.width = len;
        } else {
            self.height = len;
        }
    }

    pub fn min(&self, other: Self) -> Self {
        Self {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }

    pub fn max(&self, other: Self) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}
