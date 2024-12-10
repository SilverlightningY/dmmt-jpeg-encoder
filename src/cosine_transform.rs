pub mod separated;
pub mod simple;

pub trait Discrete8x8CosineTransformer {
    fn transform(&self, values: &mut [f32; 64]);
}
