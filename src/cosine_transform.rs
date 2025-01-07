use std::marker::{Send, Sync};
use threadpool::ThreadPool;

pub mod arai;
pub mod separated;
pub mod simple;

pub struct RawPointerWrapper(*mut f32);

unsafe impl Send for RawPointerWrapper {}
unsafe impl Sync for RawPointerWrapper {}

pub trait Discrete8x8CosineTransformer
where
    Self: 'static + Send + Sync,
{
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

    /// Applies the 8x8 discrete cosine transform (DCT) on each 64-value-block by calling the
    /// transform function, beginning each block_start_index.
    ///
    /// # Safety
    ///
    /// It requires the same preconditions as the transform function.
    unsafe fn transform_blocks_sequentially(
        &self,
        block_start: RawPointerWrapper,
        block_start_indexes: Vec<usize>,
    ) {
        for block_start_index in block_start_indexes {
            self.transform(block_start.0.add(block_start_index));
        }
    }

    /// Applies the 8x8 discrete cosine transform (DCT) for each 64-value-block on a threadpool by
    /// executing the transform function multiple times. The transformation is executed on a thread
    /// of the threadpool. The size of the jobs, executed on the pool, can be controled by the
    /// `jobs_chunk_size` parameter. If the parameter is set to 100, a single thread will transform
    /// 100 blocks in sequence.
    ///
    /// # Safety
    ///
    /// It requires the same preconditions as the transform function.
    unsafe fn transform_on_threadpool(
        &'static self,
        threadpool: &ThreadPool,
        channel: *mut f32,
        channel_length: usize,
        jobs_chunk_size: usize,
    ) {
        let block_start_indexes = (0..channel_length).step_by(64).collect::<Vec<usize>>();
        let block_start_index_chunks = block_start_indexes.chunks(jobs_chunk_size);
        for chunk in block_start_index_chunks {
            let block_start_indexes = chunk.to_vec();
            unsafe {
                let channel_start = RawPointerWrapper(channel);
                threadpool.execute(move || {
                    self.transform_blocks_sequentially(channel_start, block_start_indexes);
                });
            }
        }
    }
}
