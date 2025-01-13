use std::fmt::Debug;

use crate::image::ColorChannel;

use super::frequency_block::FrequencyBlock;

#[rustfmt::skip]
pub const QUANTIZATION_TABLE: [u8; 64] =  [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77,
    24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

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
        let buffer = self.inner_iterator.by_ref().take(64).collect::<Vec<T>>();
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
}

impl<'a, T> Quantizer<'a, T> {
    pub fn new(channel: &'a ColorChannel<T>) -> Self {
        Self { channel }
    }
}

impl<'a> Quantizer<'a, f32> {
    pub fn quantize_channel(&self) -> impl Iterator<Item = FrequencyBlock<i16>> + use<'a> {
        let data_iterator = self
            .channel
            .dots
            .iter()
            .zip(QUANTIZATION_TABLE.iter().cycle())
            .map(|(&d, &q)| (d / q as f32).round() as i16);
        BlockGroupingIterator::from(data_iterator)
    }
}
