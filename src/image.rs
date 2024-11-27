use std::iter::Sum;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;

use clap::builder::PossibleValue;
use clap::ValueEnum;

use crate::Arguments;

pub mod encoder;
pub mod ppm_parser;
pub mod transformer;

pub struct Image<T> {
    width: u16,
    height: u16,
    luma: Vec<T>,
    chroma_blue: Vec<T>,
    chroma_red: Vec<T>,
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
}

#[derive(Clone, Copy)]
pub enum ChannelIndex {
    Luma,
    ChromaBlue,
    ChromaRed,
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

struct ChannelSubsamplingInfo<'a, T> {
    image: &'a Image<T>,
    /// vertical subsampling rate
    vertical_rate: u16,
    /// horizontal subsampling rate
    horizontal_rate: u16,
    channel_index: ChannelIndex,
    /// how to sample the image
    method: ChannelSubsamplingMethod,
}

/// a potentially subsampled image iterator
pub struct ChannelRowView<'a, T> {
    subsampling_info: ChannelSubsamplingInfo<'a, T>,
    row_index: u16,
}

pub struct ChannelColumnView<'a, T> {
    subsampling_info: ChannelSubsamplingInfo<'a, T>,
    column_index: u16,
    row_index: u16,
}

impl<T> std::ops::Index<ChannelIndex> for Image<T> {
    type Output = Vec<T>;

    fn index(&self, channel_index: ChannelIndex) -> &Self::Output {
        match channel_index {
            ChannelIndex::Luma => &self.luma,
            ChannelIndex::ChromaBlue => &self.chroma_blue,
            ChannelIndex::ChromaRed => &self.chroma_red,
        }
    }
}

impl<'a, T> ChannelRowView<'a, T> {
    pub fn from_image(
        image: &'a Image<T>,
        channel_index: ChannelIndex,
        vertical_rate: u16,
        horizontal_rate: u16,
        method: ChannelSubsamplingMethod,
    ) -> Self {
        ChannelRowView {
            subsampling_info: ChannelSubsamplingInfo {
                image,
                vertical_rate,
                horizontal_rate,
                channel_index,
                method,
            },
            row_index: 0,
        }
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

impl<T> Image<T> {
    pub fn channel_view(
        &self,
        channel_index: ChannelIndex,
        vertical_rate: u16,
        horizontal_rate: u16,
        method: ChannelSubsamplingMethod,
    ) -> ChannelRowView<T> {
        ChannelRowView::from_image(self, channel_index, vertical_rate, horizontal_rate, method)
    }
}

impl<'a, T> Iterator for ChannelRowView<'a, T> {
    type Item = ChannelColumnView<'a, T>;

    fn nth(&mut self, n: usize) -> Option<ChannelColumnView<'a, T>> {
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    fn next(&mut self) -> Option<ChannelColumnView<'a, T>> {
        if self.row_index >= self.subsampling_info.image.height {
            return None;
        }
        let return_value = ChannelColumnView {
            subsampling_info: ChannelSubsamplingInfo {
                image: self.subsampling_info.image,
                vertical_rate: self.subsampling_info.vertical_rate,
                horizontal_rate: self.subsampling_info.horizontal_rate,
                channel_index: self.subsampling_info.channel_index,
                method: self.subsampling_info.method,
            },
            column_index: 0,
            row_index: self.row_index,
        };
        self.row_index += self.subsampling_info.vertical_rate;
        println!("Row Index {}", self.row_index);
        Some(return_value)
    }
}

fn average<T>(v: &[T]) -> T
where
    T: Copy + Div<Output = T> + From<u16> + Sum<T>,
{
    v.iter().copied().sum::<T>() / From::from(v.len() as _)
}

impl<'a, T> Iterator for ChannelColumnView<'a, T>
where
    T: Sized + Copy + AddAssign + DivAssign + Sum + From<u16> + Div + Div<Output = T>,
{
    type Item = T;

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.column_index >= self.subsampling_info.image.width {
            return None;
        }
        let return_value = match self.subsampling_info.method {
            ChannelSubsamplingMethod::Skip => {
                let index: usize = (self.column_index
                    + self.row_index * self.subsampling_info.image.width)
                    as usize;
                self.subsampling_info.image[self.subsampling_info.channel_index][index]
            }
            ChannelSubsamplingMethod::Average => {
                let mut acc: Vec<T> = Vec::new();
                for x in 0..self.subsampling_info.horizontal_rate {
                    let clamped_x =
                        std::cmp::min(self.subsampling_info.image.width - 1, x + self.column_index);
                    for y in 0..self.subsampling_info.vertical_rate {
                        let clamped_y = std::cmp::min(
                            self.subsampling_info.image.height - 1,
                            y + self.row_index,
                        );
                        let index: usize =
                            (clamped_x + clamped_y * self.subsampling_info.image.width) as usize;
                        acc.push(
                            self.subsampling_info.image[self.subsampling_info.channel_index][index],
                        );
                    }
                }
                average(&acc)
            }
        };
        self.column_index += self.subsampling_info.horizontal_rate;
        println!("Column Index {}", self.column_index);
        Some(return_value)
    }
}

#[cfg(test)]
mod test {
    use super::{ChannelIndex, ChannelRowView, ChannelSubsamplingMethod, Image};

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
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            luma: Vec::from(TEST_CHANNEL_ONE),
            chroma_blue: Vec::from(DUMMY_CHANNEL),
            chroma_red: Vec::from(DUMMY_CHANNEL),
        };

        let mut my_itr: ChannelRowView<f32> = ChannelRowView::from_image(
            &my_img,
            ChannelIndex::Luma,
            1,
            1,
            ChannelSubsamplingMethod::Skip,
        );

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(2)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn skip_subsampling_test() {
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            chroma_blue: Vec::from(TEST_CHANNEL_ONE),
            luma: Vec::from(DUMMY_CHANNEL),
            chroma_red: Vec::from(DUMMY_CHANNEL),
        };

        let mut my_itr: ChannelRowView<f32> = ChannelRowView::from_image(
            &my_img,
            ChannelIndex::ChromaBlue,
            1,
            2,
            ChannelSubsamplingMethod::Skip,
        );

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 7.0);
    }

    #[test]
    fn average_subsampling_test() {
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            chroma_blue: Vec::from(TEST_CHANNEL_ONE),
            luma: Vec::from(DUMMY_CHANNEL),
            chroma_red: Vec::from(DUMMY_CHANNEL),
        };

        let mut my_itr: ChannelRowView<f32> = ChannelRowView::from_image(
            &my_img,
            ChannelIndex::ChromaBlue,
            2,
            1,
            ChannelSubsamplingMethod::Average,
        );

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 12.0);
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_high() {
        #[rustfmt::skip]
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            chroma_blue: Vec::from(TEST_CHANNEL_ONE),
            luma: Vec::from(DUMMY_CHANNEL),
            chroma_red: Vec::from(DUMMY_CHANNEL),
        };

        let mut my_itr: ChannelRowView<f32> = my_img.channel_view(
            ChannelIndex::ChromaBlue,
            2,
            1,
            ChannelSubsamplingMethod::Average,
        );

        let _ = my_itr
            .nth(2)
            .expect("image should have 4 rows")
            .nth(2)
            .expect("image should have 4 columns");
    }

    #[test]
    fn repeat_border_test() {
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            chroma_blue: Vec::from(TEST_CHANNEL_ONE),
            luma: Vec::from(DUMMY_CHANNEL),
            chroma_red: Vec::from(DUMMY_CHANNEL),
        };

        let mut my_itr: ChannelRowView<f32> = ChannelRowView::from_image(
            &my_img,
            ChannelIndex::ChromaBlue,
            3,
            2,
            ChannelSubsamplingMethod::Average,
        );

        let val = my_itr
            .nth(1)
            .expect("image should have 4 rows")
            .nth(1)
            .expect("image should have 4 columns");
        assert_eq!(val, 15.5);
    }
}
