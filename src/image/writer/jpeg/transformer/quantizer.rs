use std::fmt::Debug;

use crate::image::ColorChannel;

use super::frequency_block::FrequencyBlock;

pub struct BlockGroupingIterator<S: Iterator> {
    inner_iterator: S,
}

impl<S> From<S> for BlockGroupingIterator<S>
where
    S: Iterator,
{
    fn from(inner_iterator: S) -> Self {
        Self { inner_iterator }
    }
}

impl<S, T> Iterator for BlockGroupingIterator<S>
where
    T: Debug,
    S: Iterator<Item = T>,
{
    type Item = FrequencyBlock<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer: Vec<T> = self.inner_iterator.by_ref().take(64).collect();
        if buffer.len() < 64 {
            return None;
        }
        let buffer: [T; 64] = buffer
            .try_into()
            .expect("Conversion from vector to array failed");
        Some(FrequencyBlock::new(buffer))
    }
}

pub struct Quantizer<'a, T> {
    channel: &'a ColorChannel<T>,
    quantization_table: &'a [u8; 64],
}

impl<'a, T> Quantizer<'a, T> {
    pub fn new(channel: &'a ColorChannel<T>, quantization_table: &'a [u8; 64]) -> Self {
        Self {
            channel,
            quantization_table,
        }
    }
}

impl<'a> Quantizer<'a, f32> {
    pub fn quantize_channel(&self) -> impl Iterator<Item = FrequencyBlock<i16>> + use<'a> {
        let data_iterator = self
            .channel
            .dots
            .iter()
            .zip(self.quantization_table.iter().cycle())
            .map(|(&d, &q)| (d / q as f32).round() as i16);
        BlockGroupingIterator::from(data_iterator)
    }
}
