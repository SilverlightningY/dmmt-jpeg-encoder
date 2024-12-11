pub mod arai;
pub mod separated;
pub mod simple;

pub trait Discrete8x8CosineTransformer {
    /// Applies the 8x8 discrete cosine transform (DCT).
    ///
    /// The transformation is done in place on the coniguous data structure behind the mutable raw
    /// pointer.
    ///
    /// # Safety
    ///
    /// This function transforms array behind block_start in place. It processes only 8x8 (64)
    /// values. The caller has to make sure, the array has a length of at least 64 values. If it is
    /// used from multiple threads at the same time, the ranges must not overlap each other.
    /// Otherwise the result can not be foreseen and is considered undefined.
    unsafe fn transform(&self, block_start: *mut f32);
}
