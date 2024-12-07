use std::cmp;
use std::collections::VecDeque;
use std::iter::Sum;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;

use clap::builder::PossibleValue;
use clap::ValueEnum;

use crate::huffman::SymbolCodeLength;
use crate::Arguments;

pub mod encoder;
pub mod ppm_parser;
pub mod transformer;

pub struct Image<T> {
    width: u16,
    height: u16,
    luma: ColorChannel<T>,
    chroma_blue: ColorChannel<T>,
    chroma_red: ColorChannel<T>,
}

impl<T> Image<T> {
    pub fn new(
        width: u16,
        height: u16,
        luma: Vec<T>,
        chroma_blue: Vec<T>,
        chroma_red: Vec<T>,
    ) -> Self {
        Self {
            width,
            height,
            luma: ColorChannel {
                width,
                height,
                dots: luma,
            },
            chroma_blue: ColorChannel {
                width,
                height,
                dots: chroma_blue,
            },
            chroma_red: ColorChannel {
                width,
                height,
                dots: chroma_red,
            },
        }
    }

    pub fn luma_channel(&self) -> &ColorChannel<T> {
        &self.luma
    }

    pub fn chroma_red_channel(&self) -> &ColorChannel<T> {
        &self.chroma_red
    }

    pub fn chroma_blue_channel(&self) -> &ColorChannel<T> {
        &self.chroma_blue
    }
}

pub struct ColorChannel<T> {
    dots: Vec<T>,
    width: u16,
    height: u16,
}

impl<T> ColorChannel<T>
where
    T: Clone + Copy,
{
    fn dot(&self, column_index: u16, row_index: u16) -> T {
        let index = column_index + row_index * self.width;
        self.dots[index as usize]
    }

    fn rect(&self, column_index: u16, row_index: u16, width: u16, height: u16) -> Vec<T> {
        let rect_length = width * height;
        let mut acc: Vec<T> = Vec::with_capacity(rect_length as usize);
        let last_column_index = self.width - 1;
        let last_row_index = self.height - 1;
        for x in 0..width {
            let current_column_index = cmp::min(last_column_index, x + column_index);
            for y in 0..height {
                let current_row_index = cmp::min(last_row_index, y + row_index);
                acc.push(self.dot(current_column_index, current_row_index));
            }
        }
        acc
    }

    pub fn subsampling_iter<'a>(
        &'a self,
        subsampling_config: &'a ChannelSubsamplingConfig,
    ) -> ChannelRowView<'a, T> {
        ChannelRowView {
            channel: self,
            subsampling_config,
            row_index: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChromaSubsamplingPreset {
    P444,
    P422,
    P420,
}

impl ValueEnum for ChromaSubsamplingPreset {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::P444, Self::P422, Self::P420]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::P444 => Some(PossibleValue::new("P444")),
            Self::P422 => Some(PossibleValue::new("P422")),
            Self::P420 => Some(PossibleValue::new("P420")),
        }
    }
}

impl ChromaSubsamplingPreset {
    fn horizontal_rate(&self) -> u8 {
        match self {
            ChromaSubsamplingPreset::P444 => 1,
            ChromaSubsamplingPreset::P422 => 2,
            ChromaSubsamplingPreset::P420 => 2,
        }
    }

    fn vertical_rate(&self) -> u8 {
        match self {
            ChromaSubsamplingPreset::P444 => 1,
            ChromaSubsamplingPreset::P422 => 1,
            ChromaSubsamplingPreset::P420 => 2,
        }
    }
}

pub struct OutputImage {
    width: u16,
    height: u16,
    chroma_subsampling_preset: ChromaSubsamplingPreset,
    bits_per_channel: u8,
    luma_ac_huffman: Vec<SymbolCodeLength>,
    luma_dc_huffman: Vec<SymbolCodeLength>,
    chroma_ac_huffman: Vec<SymbolCodeLength>,
    chroma_dc_huffman: Vec<SymbolCodeLength>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelSubsamplingMethod {
    Skip,
    Average,
}

impl ValueEnum for ChannelSubsamplingMethod {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Skip, Self::Average]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Skip => Some(PossibleValue::new("Skip")),
            Self::Average => Some(PossibleValue::new("Average")),
        }
    }
}

pub struct ChannelSubsamplingConfig {
    /// vertical subsampling rate
    pub vertical_rate: u16,
    /// horizontal subsampling rate
    pub horizontal_rate: u16,
    /// how to sample the image
    pub method: ChannelSubsamplingMethod,
}

/// a potentially subsampled image iterator
pub struct ChannelRowView<'a, T> {
    subsampling_config: &'a ChannelSubsamplingConfig,
    row_index: u16,
    channel: &'a ColorChannel<T>,
}

impl<'a, T> ChannelRowView<'a, T> {
    pub fn into_square_iter(self, square_size: usize) -> ChannelSquareIterator<'a, T> {
        ChannelSquareIterator {
            row_view: self,
            square_buffer: VecDeque::new(),
            square_size,
        }
    }
}

impl<'a, T> Iterator for ChannelRowView<'a, T> {
    type Item = ChannelColumnView<'a, T>;

    fn nth(&mut self, n: usize) -> Option<ChannelColumnView<'a, T>> {
        self.row_index += self.subsampling_config.vertical_rate * n as u16;
        if self.row_index >= self.channel.height {
            return None;
        }
        let return_value = ChannelColumnView {
            subsampling_config: self.subsampling_config,
            column_index: 0,
            row_index: self.row_index,
            channel: self.channel,
        };
        self.row_index += self.subsampling_config.vertical_rate;
        Some(return_value)
    }

    fn next(&mut self) -> Option<ChannelColumnView<'a, T>> {
        self.nth(0)
    }
}

pub struct ChannelColumnView<'a, T> {
    subsampling_config: &'a ChannelSubsamplingConfig,
    column_index: u16,
    row_index: u16,
    channel: &'a ColorChannel<T>,
}

impl<'a, T> Iterator for ChannelColumnView<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    type Item = T;

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.column_index += self.subsampling_config.horizontal_rate * n as u16;
        if self.column_index >= self.channel.width {
            return None;
        }
        let return_value = match self.subsampling_config.method {
            ChannelSubsamplingMethod::Skip => self.channel.dot(self.column_index, self.row_index),
            ChannelSubsamplingMethod::Average => {
                let width = self.subsampling_config.horizontal_rate;
                let height = self.subsampling_config.vertical_rate;
                let subsampling_rect =
                    self.channel
                        .rect(self.column_index, self.row_index, width, height);
                average(&subsampling_rect)
            }
        };
        self.column_index += self.subsampling_config.horizontal_rate;
        Some(return_value)
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }
}

pub struct TransformationOptions {
    pub chroma_subsampling_preset: ChromaSubsamplingPreset,
    pub bits_per_channel: u8,
    pub chroma_subsampling_method: ChannelSubsamplingMethod,
}

impl From<&Arguments> for TransformationOptions {
    fn from(value: &Arguments) -> Self {
        Self {
            chroma_subsampling_preset: value.chroma_subsampling_preset,
            bits_per_channel: value.bits_per_channel,
            chroma_subsampling_method: value.chroma_subsampling_method,
        }
    }
}

fn average<T>(v: &[T]) -> T
where
    T: Copy + Div<Output = T> + From<u16> + Sum<T>,
{
    v.iter().copied().sum::<T>() / From::from(v.len() as _)
}

pub struct ChannelSquareIterator<'a, T> {
    row_view: ChannelRowView<'a, T>,
    square_buffer: VecDeque<Vec<T>>,
    square_size: usize,
}

impl<'a, T> ChannelSquareIterator<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    fn read_next_square_size_rows(&mut self) {
        for _ in 0..self.square_size {
            self.read_next_row();
            self.pad_row();
        }
        self.pad_all_blocks_in_buffer();
    }

    fn read_next_row(&mut self) {
        if let Some(row) = self.row_view.next() {
            for (index, value) in row.enumerate() {
                let square_index = index / self.square_size;
                self.fill_buffer_to_length(square_index + 1);
                self.square_buffer[square_index].push(value);
            }
        }
    }

    fn pad_all_blocks_in_buffer(&mut self) {
        for square in self.square_buffer.iter_mut() {
            Self::pad_block(square, self.square_size);
        }
    }

    fn fill_buffer_to_length(&mut self, length: usize) {
        let old_length = self.square_buffer.len();
        let difference = length.saturating_sub(old_length);
        if difference == 0 {
            return;
        }
        for _ in 0..difference {
            self.square_buffer
                .push_back(self.create_empty_square_buffer());
        }
    }

    fn create_empty_square_buffer(&self) -> Vec<T> {
        let capacity = self.square_size * self.square_size;
        Vec::with_capacity(capacity)
    }

    fn pad_block(block: &mut Vec<T>, square_size: usize) {
        let row_count = block.len() / square_size;
        let number_of_rows_to_extend = square_size - row_count;
        if number_of_rows_to_extend == 0 {
            return;
        }
        let last_row_start_index = block.len() - square_size;
        let last_row = &block[last_row_start_index..block.len()].to_owned();
        for _ in 0..number_of_rows_to_extend {
            block.extend(last_row);
        }
    }

    fn pad_row(&mut self) {
        let last_square = self
            .square_buffer
            .back_mut()
            .expect("Square buffer must not be empty to pad row of last square");
        let number_of_values_in_last_row = last_square.len() % self.square_size;
        if number_of_values_in_last_row == 0 {
            return;
        }
        let number_of_values_to_fill_up = self.square_size - number_of_values_in_last_row;
        let last_value = *last_square
            .last()
            .expect("Last square in buffer must not be empty");
        for _ in 0..number_of_values_to_fill_up {
            last_square.push(last_value);
        }
    }
}

impl<'a, T> Iterator for ChannelSquareIterator<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.square_buffer.is_empty() {
            self.read_next_square_size_rows();
        }
        self.square_buffer.pop_front()
    }
    // add code here
}

#[cfg(test)]
mod test {
    use crate::image::ChannelSubsamplingConfig;

    use super::{ChannelSubsamplingMethod, Image};

    #[rustfmt::skip]
    const TEST_CHANNEL_ONE: &[f32] = &[
         1.0,  2.0,  3.0,  4.0,
         5.0,  6.0,  7.0,  8.0,
         9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];

    const TEST_CHANNEL_TWO: &[f32] = &[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0,
        32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0,
        47.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 60.0, 61.0,
        62.0, 63.0, 64.0,
    ];

    const DUMMY_CHANNEL: &[f32] = &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ];

    #[test]
    fn no_subsampling_test() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
            Vec::from(DUMMY_CHANNEL),
        );

        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let mut my_itr = my_img.luma_channel().subsampling_iter(&subsampling_config);

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(2)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn skip_subsampling_test() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(DUMMY_CHANNEL),
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
        );

        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let mut my_itr = my_img
            .chroma_blue_channel()
            .subsampling_iter(&subsampling_config);

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn average_subsampling_test() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(DUMMY_CHANNEL),
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
        );

        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 2,
            method: ChannelSubsamplingMethod::Average,
        };
        let mut my_itr = my_img
            .chroma_blue_channel()
            .subsampling_iter(&subsampling_config);

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 12.0);
    }

    #[test]
    fn out_of_bounds_high() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(DUMMY_CHANNEL),
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
        );

        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Average,
        };
        let mut my_itr = my_img
            .chroma_blue_channel()
            .subsampling_iter(&subsampling_config);

        let option = my_itr.nth(2).expect("image should have 4 rows").nth(2);
        assert!(option.is_none(), "Read out of bounds should return none");
    }

    #[test]
    fn repeat_border_test() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(DUMMY_CHANNEL),
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
        );

        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 3,
            method: ChannelSubsamplingMethod::Average,
        };
        let mut my_itr = my_img
            .chroma_blue_channel()
            .subsampling_iter(&subsampling_config);

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 15.5);
    }

    #[test]
    fn test_block_iter_with_single_fit_image() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
            Vec::from(DUMMY_CHANNEL),
        );
        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let mut block_iterator = my_img
            .luma_channel()
            .subsampling_iter(&subsampling_config)
            .into_square_iter(4);
        let block = block_iterator
            .next()
            .expect("Image should fit into one block");
        assert_eq!(block.len(), 16, "Block must have 4x4 fields");
        for (&actual, &expected) in block.iter().zip(TEST_CHANNEL_ONE) {
            assert_eq!(actual, expected, "Value does not match");
        }
    }

    #[test]
    fn test_square_iter_with_single_and_too_small_image() {
        let my_img: Image<f32> = Image::new(
            4,
            4,
            Vec::from(TEST_CHANNEL_ONE),
            Vec::from(DUMMY_CHANNEL),
            Vec::from(DUMMY_CHANNEL),
        );
        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let square_size = 6;
        let mut block_iterator = my_img
            .luma_channel()
            .subsampling_iter(&subsampling_config)
            .into_square_iter(square_size);
        let block = block_iterator
            .next()
            .expect("Image should fit into one block");
        for ((index, &a), (&b, &c)) in block.iter().skip(3).step_by(square_size).enumerate().zip(
            block
                .iter()
                .skip(4)
                .step_by(square_size)
                .zip(block.iter().skip(5).step_by(square_size)),
        ) {
            assert!(
                a == b && b == c,
                "Padding in row {} is not applied correctly",
                index + 1
            );
        }
        assert_eq!(
            block.len(),
            square_size * square_size,
            "Block must have {0}x{0} fields",
            square_size
        );
    }

    #[test]
    fn test_square_iter_with_large_image() {
        let my_img: Image<f32> = Image::new(
            8,
            8,
            Vec::from(TEST_CHANNEL_TWO),
            Vec::from(DUMMY_CHANNEL),
            Vec::from(DUMMY_CHANNEL),
        );
        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let square_size = 4;
        let mut block_iterator = my_img
            .luma_channel()
            .subsampling_iter(&subsampling_config)
            .into_square_iter(square_size);
        let block = block_iterator
            .nth(3)
            .expect("Image should be cut into 4 squares");
        let last_row_start_index = TEST_CHANNEL_TWO.len() - square_size;
        let expected_last_row = &TEST_CHANNEL_TWO[last_row_start_index..TEST_CHANNEL_TWO.len()];
        for (index, (&acutal, &expected)) in block
            .iter()
            .skip(square_size * (square_size - 1))
            .zip(expected_last_row.iter())
            .enumerate()
        {
            assert_eq!(acutal, expected, "Item at index {} does not match", index);
        }
        assert_eq!(
            block.len(),
            square_size * square_size,
            "Block must have {0}x{0} fields",
            square_size
        );
    }

    #[test]
    fn test_square_iter_with_large_image_with_padding() {
        let my_img: Image<f32> = Image::new(
            8,
            8,
            Vec::from(TEST_CHANNEL_TWO),
            Vec::from(DUMMY_CHANNEL),
            Vec::from(DUMMY_CHANNEL),
        );
        let subsampling_config = ChannelSubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: ChannelSubsamplingMethod::Skip,
        };
        let square_size = 7;
        let mut block_iterator = my_img
            .luma_channel()
            .subsampling_iter(&subsampling_config)
            .into_square_iter(square_size);
        let block = block_iterator
            .nth(3)
            .expect("Image should be cut into 4 squares");
        assert_eq!(
            block.len(),
            square_size * square_size,
            "Block must have {0}x{0} fields",
            square_size
        );
        let mut previous_value = block.last().unwrap().to_owned();
        for (index, &value) in block.iter().enumerate() {
            let x = index % square_size;
            let y = index / square_size;
            assert_eq!(
                value, previous_value,
                "Padded value does not match at x = {}, y = {}",
                x, y
            );
            previous_value = value;
        }
    }
}
