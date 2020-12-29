pub fn float_eq(a: f32, b:f32, variation: f32) -> bool {
    f32::abs(a - b) < variation
}