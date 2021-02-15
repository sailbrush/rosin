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

    pub fn main(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.left + self.right
        } else {
            self.top + self.bottom
        }
    }

    pub fn cross(&self, dir: FlexDirection) -> f32 {
        if !dir.is_row() {
            self.left + self.right
        } else {
            self.top + self.bottom
        }
    }

    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
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

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Bounds {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Bounds {
    pub fn min_main(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.min_width
        } else {
            self.min_height
        }
    }

    pub fn max_main(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.max_width
        } else {
            self.max_height
        }
    }
}
