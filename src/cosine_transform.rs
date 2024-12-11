pub mod arai;
pub mod separated;
pub mod simple;

pub trait Discrete8x8CosineTransformer {
    fn transform(&self, image_slice: &mut [f32], row_length: usize);
}
