use std::{
    cmp,
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
}

impl<'a, T> Subsampler<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T> + Default,
{
    pub fn subsample_to_square_structure(&'a self, square_size: usize) -> Vec<T> {
        self.subsampling_iter()
            .into_square_resorter(square_size)
            .resort()
    }
}

/// a potentially subsampled image iterator
pub struct ChannelRowView<'a, T> {
    subsampling_config: &'a SubsamplingConfig,
    row_index: u16,
    subsampler: &'a Subsampler<'a, T>,
}

impl<'a, T> ChannelRowView<'a, T>
where
    T: Copy + Default,
{
    pub fn into_square_resorter(self, square_size: usize) -> ChannelSquareResorter<'a, T> {
        let channel_width = self.subsampler.color_channel.width;
        let channel_height = self.subsampler.color_channel.height;
        let subsampled_width = channel_width / self.subsampling_config.horizontal_rate;
        let subsampled_height = channel_height / self.subsampling_config.vertical_rate;
        let number_of_items = subsampled_width as usize * subsampled_height as usize;
        ChannelSquareResorter::new(
            self,
            square_size,
            number_of_items,
            subsampled_width as usize,
        )
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

pub struct ChannelSquareResorter<'a, T> {
    row_view: ChannelRowView<'a, T>,
    result_buffer: Vec<T>,
    square_size: usize,
    square_length: usize,
    number_of_items_per_block_row: usize,
}

impl<'a, T> ChannelSquareResorter<'a, T>
where
    T: Copy + Default,
{
    fn new(
        row_view: ChannelRowView<'a, T>,
        square_size: usize,
        number_of_items: usize,
        row_length: usize,
    ) -> Self {
        let number_of_items_per_block_row = row_length * square_size;
        Self {
            row_view,
            square_size,
            result_buffer: vec![T::default(); number_of_items],
            square_length: square_size * square_size,
            number_of_items_per_block_row,
        }
    }
}

impl<T> ChannelSquareResorter<'_, T> {
    fn calculate_item_index_for_square(
        &mut self,
        square_column_index: usize,
        square_row_index: usize,
        x: usize,
        y: usize,
    ) -> usize {
        let first_column_index = square_column_index * self.square_length;
        let first_row_index = square_row_index * self.number_of_items_per_block_row;
        let row_start_index = y * self.square_size;
        first_row_index + first_column_index + row_start_index + x
    }
}

impl<T> ChannelSquareResorter<'_, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    pub fn resort(mut self) -> Vec<T> {
        self.read_all_rows();
        self.result_buffer
    }

    fn read_all_rows(&mut self) {
        let mut row_index = 0;
        while let Some(row) = self.row_view.next() {
            self.insert_row_into_output_buffer(row_index, row);
            row_index += 1;
        }
    }

    fn insert_row_into_output_buffer(&mut self, row_index: usize, row: impl Iterator<Item = T>) {
        for (index, value) in row.enumerate() {
            let square_column_index = index / self.square_size;
            let x = index % self.square_size;
            let square_row_index = row_index / self.square_size;
            let y = row_index % self.square_size;
            let item_index =
                self.calculate_item_index_for_square(square_column_index, square_row_index, x, y);
            self.result_buffer[item_index] = value;
        }
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

    const TEST_CHANNEL_TWO: &[f32; 64] = &[
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
        let block = subsampler.subsample_to_square_structure(4);
        assert_eq!(block.len(), 16, "Block must have 4x4 fields");
        for (&actual, &expected) in block.iter().zip(TEST_CHANNEL_ONE) {
            assert_eq!(actual, expected, "Value does not match");
        }
    }

    #[test]
    fn test_square_resorter_with_1x1_subsampling() {
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
        let resorted_channel = subsampler.subsample_to_square_structure(4);
        let expected: &[f32; 64] = &[
            1.0, 2.0, 3.0, 4.0, 9.0, 10.0, 11.0, 12.0, 17.0, 18.0, 19.0, 20.0, 25.0, 26.0, 27.0,
            28.0, 5.0, 6.0, 7.0, 8.0, 13.0, 14.0, 15.0, 16.0, 21.0, 22.0, 23.0, 24.0, 29.0, 30.0,
            31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 41.0, 42.0, 43.0, 44.0, 49.0, 50.0, 51.0, 52.0,
            57.0, 58.0, 59.0, 60.0, 37.0, 38.0, 39.0, 40.0, 45.0, 46.0, 47.0, 48.0, 53.0, 54.0,
            55.0, 56.0, 61.0, 62.0, 63.0, 64.0,
        ];
        assert_eq!(
            resorted_channel.len(),
            expected.len(),
            "Length of resorted channel does not match"
        );
        for (&actual, &expected) in resorted_channel.iter().zip(expected) {
            assert_eq!(actual, expected, "Value does not match");
        }
    }

    #[test]
    fn test_square_resorter_with_2x2_subsampling() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_TWO),
            width: 8,
            height: 8,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 2,
            vertical_rate: 2,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let resorted_channel = subsampler.subsample_to_square_structure(4);
        let expected: &[f32; 16] = &[
            1.0, 3.0, 5.0, 7.0, 17.0, 19.0, 21.0, 23.0, 33.0, 35.0, 37.0, 39.0, 49.0, 51.0, 53.0,
            55.0,
        ];
        assert_eq!(
            resorted_channel.len(),
            expected.len(),
            "Length of resorted channel does not match"
        );
        for (&actual, &expected) in resorted_channel.iter().zip(expected) {
            assert_eq!(actual, expected, "Value does not match");
        }
    }

    #[test]
    fn test_square_resorter_with_1x2_subsampling() {
        let color_channel = ColorChannel {
            dots: Vec::from(TEST_CHANNEL_TWO),
            width: 8,
            height: 8,
        };
        let subsampling_config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 2,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(&color_channel, &subsampling_config);
        let resorted_channel = subsampler.subsample_to_square_structure(4);
        let expected: &[f32; 32] = &[
            1.0, 2.0, 3.0, 4.0, 17.0, 18.0, 19.0, 20.0, 33.0, 34.0, 35.0, 36.0, 49.0, 50.0, 51.0,
            52.0, 5.0, 6.0, 7.0, 8.0, 21.0, 22.0, 23.0, 24.0, 37.0, 38.0, 39.0, 40.0, 53.0, 54.0,
            55.0, 56.0,
        ];
        assert_eq!(
            resorted_channel.len(),
            expected.len(),
            "Length of resorted channel does not match"
        );
        for (&actual, &expected) in resorted_channel.iter().zip(expected) {
            assert_eq!(actual, expected, "Value does not match");
        }
    }
}
