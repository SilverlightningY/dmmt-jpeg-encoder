pub mod separated;
pub mod simple;

pub trait Discrete8x8CosineTransformer {
    fn transform(values: &[f32; 64]) -> [f32; 64];
}
