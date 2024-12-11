pub mod arai;
pub mod separated;
pub mod simple;

pub trait Discrete8x8CosineTransformer {
    unsafe fn transform(&self, block_start: *mut f32);
}
