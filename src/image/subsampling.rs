use std::{
    cmp,
    collections::VecDeque,
    iter::Sum,
    ops::{AddAssign, Div, DivAssign},
};

use clap::{builder::PossibleValue, ValueEnum};

use super::ColorChannel;

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
    pub fn horizontal_rate(&self) -> u8 {
        match self {
            ChromaSubsamplingPreset::P444 => 1,
            ChromaSubsamplingPreset::P422 => 2,
            ChromaSubsamplingPreset::P420 => 2,
        }
    }

    pub fn vertical_rate(&self) -> u8 {
        match self {
            ChromaSubsamplingPreset::P444 => 1,
            ChromaSubsamplingPreset::P422 => 1,
            ChromaSubsamplingPreset::P420 => 2,
        }
    }

    pub fn method(&self) -> SubsamplingMethod {
        match self {
            ChromaSubsamplingPreset::P444 => SubsamplingMethod::Skip,
            _ => SubsamplingMethod::Average,
        }
    }
}

pub enum SubsamplingMethod {
    Skip,
    Average,
}

pub struct SubsamplingConfig {
    /// vertical subsampling rate
    pub vertical_rate: u16,
    /// horizontal subsampling rate
    pub horizontal_rate: u16,
    /// how to sample the image
    pub method: SubsamplingMethod,
}

impl From<ChromaSubsamplingPreset> for SubsamplingConfig {
    fn from(value: ChromaSubsamplingPreset) -> Self {
        Self {
            vertical_rate: value.vertical_rate() as u16,
            horizontal_rate: value.horizontal_rate() as u16,
            method: value.method(),
        }
    }
}

pub struct Subsampler<'a, T> {
    color_channel: &'a ColorChannel<T>,
    subsampling_config: &'a SubsamplingConfig,
}

impl<'a, T> Subsampler<'a, T> {
    pub fn new(
        color_channel: &'a ColorChannel<T>,
        subsampling_config: &'a SubsamplingConfig,
    ) -> Self {
        Self {
            color_channel,
            subsampling_config,
        }
    }
}

impl<'a, T> Subsampler<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    fn dot(&self, column_index: u16, row_index: u16) -> T {
        let index: usize =
            column_index as usize + row_index as usize * self.color_channel.width as usize;
        self.color_channel.dots[index]
    }

    fn rect(&self, column_index: u16, row_index: u16, width: u16, height: u16) -> Vec<T> {
        let rect_length = width * height;
        let mut acc: Vec<T> = Vec::with_capacity(rect_length as usize);
        let color_channel = self.color_channel;
        let last_column_index = color_channel.width - 1;
        let last_row_index = color_channel.height - 1;
        for x in 0..width {
            let current_column_index = cmp::min(last_column_index, x + column_index);
            for y in 0..height {
                let current_row_index = cmp::min(last_row_index, y + row_index);
                acc.push(self.dot(current_column_index, current_row_index));
            }
        }
        acc
    }

    pub fn subsampling_iter(&'a self) -> ChannelRowView<'a, T> {
        ChannelRowView {
            subsampler: self,
            subsampling_config: self.subsampling_config,
            row_index: 0,
        }
    }

    pub fn subsample_to_square_structure(&self, square_size: usize) -> Vec<T> {
        self.subsampling_iter()
            .into_square_iter(square_size)
            .flatten()
            .collect()
    }
}

/// a potentially subsampled image iterator
pub struct ChannelRowView<'a, T> {
    subsampling_config: &'a SubsamplingConfig,
    row_index: u16,
    subsampler: &'a Subsampler<'a, T>,
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
        if self.row_index >= self.subsampler.color_channel.height {
            return None;
        }
        let return_value = ChannelColumnView {
            subsampling_config: self.subsampling_config,
            column_index: 0,
            row_index: self.row_index,
            subsampler: self.subsampler,
        };
        self.row_index += self.subsampling_config.vertical_rate;
        Some(return_value)
    }

    fn next(&mut self) -> Option<ChannelColumnView<'a, T>> {
        self.nth(0)
    }
}

pub struct ChannelColumnView<'a, T> {
    subsampling_config: &'a SubsamplingConfig,
    column_index: u16,
    row_index: u16,
    subsampler: &'a Subsampler<'a, T>,
}

impl<T> Iterator for ChannelColumnView<'_, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    type Item = T;

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.column_index += self.subsampling_config.horizontal_rate * n as u16;
        if self.column_index >= self.subsampler.color_channel.width {
            return None;
        }
        let return_value = match self.subsampling_config.method {
            SubsamplingMethod::Skip => self.subsampler.dot(self.column_index, self.row_index),
            SubsamplingMethod::Average => {
                let width = self.subsampling_config.horizontal_rate;
                let height = self.subsampling_config.vertical_rate;
                let subsampling_rect =
                    self.subsampler
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

impl<T> ChannelSquareIterator<'_, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    fn read_next_square_size_rows(&mut self) {
        for _ in 0..self.square_size {
            self.read_next_row();
        }
        self.pad_all_blocks_in_buffer();
    }

    fn read_next_row(&mut self) {
        if let Some(row) = self.row_view.next() {
            self.insert_row_into_square_buffers(row);
            self.pad_row();
        }
    }

    fn insert_row_into_square_buffers(&mut self, row: impl Iterator<Item = T>) {
        for (index, value) in row.enumerate() {
            let square_index = index / self.square_size;
            self.fill_buffer_to_length(square_index + 1);
            self.square_buffer[square_index].push(value);
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

impl<T> Iterator for ChannelSquareIterator<'_, T>
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
}

#[cfg(test)]
mod test {
    use super::{ColorChannel, Subsampler, SubsamplingConfig, SubsamplingMethod};

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

    #[test]
    fn no_subsampling_test() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };

        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut my_itr = subsampler.subsampling_iter();

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(2)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn skip_subsampling_test() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut my_itr = subsampler.subsampling_iter();

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn average_subsampling_test() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 2,
            method: SubsamplingMethod::Average,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut my_itr = subsampler.subsampling_iter();

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 12.0);
    }

    #[test]
    fn out_of_bounds_high() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 1,
            method: SubsamplingMethod::Average,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut my_itr = subsampler.subsampling_iter();

        let option = my_itr.nth(2).expect("image should have 4 rows").nth(2);
        assert!(option.is_none(), "Read out of bounds should return none");
    }

    #[test]
    fn repeat_border_test() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 3,
            method: SubsamplingMethod::Average,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut my_itr = subsampler.subsampling_iter();

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 15.5);
    }

    #[test]
    fn test_block_iter_with_single_fit_image() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let mut block_iterator = subsampler.subsampling_iter().into_square_iter(4);
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
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_ONE),
            width: 4,
            height: 4,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let square_size = 6;
        let mut block_iterator = subsampler.subsampling_iter().into_square_iter(square_size);
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
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_TWO),
            width: 8,
            height: 8,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let square_size = 4;
        let mut block_iterator = subsampler.subsampling_iter().into_square_iter(square_size);
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
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_TWO),
            width: 8,
            height: 8,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let square_size = 7;
        let mut block_iterator = subsampler.subsampling_iter().into_square_iter(square_size);
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
