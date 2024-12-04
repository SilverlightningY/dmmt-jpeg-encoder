use std::cmp;
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
    fn new(width: u16, height: u16, luma: Vec<T>, chroma_blue: Vec<T>, chroma_red: Vec<T>) -> Self {
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

    fn luma_channel(&self) -> &ColorChannel<T> {
        &self.luma
    }

    fn chroma_red_channel(&self) -> &ColorChannel<T> {
        &self.chroma_red
    }

    fn chroma_blue_channel(&self) -> &ColorChannel<T> {
        &self.chroma_blue
    }
}

struct ColorChannel<T> {
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

    fn subsampling_iter<'a>(
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
    subsampling_method: ChannelSubsamplingMethod,
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
    vertical_rate: u16,
    /// horizontal subsampling rate
    horizontal_rate: u16,
    /// how to sample the image
    method: ChannelSubsamplingMethod,
}

/// a potentially subsampled image iterator
struct ChannelRowView<'a, T> {
    subsampling_config: &'a ChannelSubsamplingConfig,
    row_index: u16,
    channel: &'a ColorChannel<T>,
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

struct ChannelColumnView<'a, T> {
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
}
