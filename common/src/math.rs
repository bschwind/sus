pub trait NormalizedInt {
    fn normalized(&self) -> f32;
}

impl NormalizedInt for i16 {
    fn normalized(&self) -> f32 {
        if *self < 0 {
            -(*self as f32) / i16::MIN as f32
        } else {
            *self as f32 / i16::MAX as f32
        }
    }
}

impl NormalizedInt for i8 {
    fn normalized(&self) -> f32 {
        if *self < 0 {
            -(*self as f32) / i8::MIN as f32
        } else {
            *self as f32 / i8::MAX as f32
        }
    }
}
