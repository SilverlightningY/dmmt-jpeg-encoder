use std::fmt::Debug;

use crate::image::ColorChannel;

use super::frequency_block::FrequencyBlock;

#[rustfmt::skip]
const QUANTIZATION_TABLE: [f32;64] =  [
    16.0, 11.0, 10.0, 16.0, 24.0, 40.0, 51.0, 61.0,
    12.0, 12.0, 14.0, 19.0, 26.0, 58.0, 60.0, 55.0,
    14.0, 13.0, 16.0, 24.0, 40.0, 57.0, 69.0, 56.0,
    14.0, 17.0, 22.0, 29.0, 51.0, 87.0, 80.0, 62.0,
    18.0, 22.0, 37.0, 56.0, 68.0, 109.0, 103.0, 77.0,
    24.0, 35.0, 55.0, 64.0, 81.0, 104.0, 113.0, 92.0,
    49.0, 64.0, 78.0, 87.0, 103.0, 121.0, 120.0, 101.0,
    72.0, 92.0, 95.0, 98.0, 112.0, 100.0, 103.0, 99.0,
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
            .map(|(&d, &q)| (d / q).round() as i16);
        BlockGroupingIterator::from(data_iterator)
    }
}
