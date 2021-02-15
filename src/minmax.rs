pub trait MinMax<In, Out> {
    fn maybe_min(self, rhs: In) -> Out;
    fn maybe_max(self, rhs: In) -> Out;
}

impl MinMax<f32, f32> for f32 {
    fn maybe_min(self, rhs: f32) -> f32 {
        self.min(rhs)
    }

    fn maybe_max(self, rhs: f32) -> f32 {
        self.max(rhs)
    }
}

impl MinMax<Option<f32>, f32> for f32 {
    fn maybe_min(self, rhs: Option<f32>) -> f32 {
        self.min(rhs.unwrap_or(f32::NAN))
    }

    fn maybe_max(self, rhs: Option<f32>) -> f32 {
        self.max(rhs.unwrap_or(f32::NAN))
    }
}

impl MinMax<f32, f32> for Option<f32> {
    fn maybe_min(self, rhs: f32) -> f32 {
        self.unwrap_or(f32::NAN).min(rhs)
    }

    fn maybe_max(self, rhs: f32) -> f32 {
        self.unwrap_or(f32::NAN).max(rhs)
    }
}

impl MinMax<Option<f32>, f32> for Option<f32> {
    fn maybe_min(self, rhs: Option<f32>) -> f32 {
        self.unwrap_or(f32::NAN).min(rhs.unwrap_or(f32::NAN))
    }

    fn maybe_max(self, rhs: Option<f32>) -> f32 {
        self.unwrap_or(f32::NAN).max(rhs.unwrap_or(f32::NAN))
    }
}
