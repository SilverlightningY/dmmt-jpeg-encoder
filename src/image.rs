pub struct Image<T> {
    width: u16,
    height: u16,
    luma: Vec<T>,
    chroma_blue: Vec<T>,
    chroma_red: Vec<T>,
}

#[derive(Clone, Copy)]
pub enum ChannelIndex {
    Luma,
    ChromaBlue,
    ChromaRed,
}

#[derive(Clone, Copy)]
pub enum ChannelSubsamplingMethod {
    Skip,
    Average,
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
pub struct ChannelView<'a, T> {
    image: ChannelSubsamplingInfo<'a, T>,
    /// which image row this iterator is on
    vertical_pos: i16,
}

pub struct ChannelColumnView<'a, T> {
    image: ChannelSubsamplingInfo<'a, T>,
    /// which image column this iterator is on
    horizontal_pos: i16,
    vertical_pos: i16,
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

impl<'a, T> ChannelView<'a, T> {
    pub fn from_image(
        image: &'a Image<T>,
        channel_index: ChannelIndex,
        vertical_rate: u16,
        horizontal_rate: u16,
        method: ChannelSubsamplingMethod,
    ) -> Self {
        ChannelView {
            image: ChannelSubsamplingInfo {
                image,
                vertical_rate,
                horizontal_rate,
                channel_index,
                method,
            },
            vertical_pos: -(vertical_rate as i16),
        }
    }
}

impl<'a, T> Iterator for ChannelView<'a, T> {
    type Item = ChannelColumnView<'a, T>;
    fn nth(&mut self, n: usize) -> Option<ChannelColumnView<'a, T>> {
        if self.vertical_pos + (self.image.vertical_rate as usize * (n + 1)) as i16
            >= self.image.image.height as i16
        {
            return None;
        }
        self.vertical_pos += (self.image.vertical_rate as usize * (n + 1)) as i16;
        Some(ChannelColumnView {
            image: ChannelSubsamplingInfo {
                image: self.image.image,
                vertical_rate: self.image.vertical_rate,
                horizontal_rate: self.image.horizontal_rate,
                channel_index: self.image.channel_index,
                method: self.image.method,
            },
            horizontal_pos: -(self.image.horizontal_rate as i16),
            vertical_pos: self.vertical_pos,
        })
    }
    fn next(&mut self) -> Option<ChannelColumnView<'a, T>> {
        self.nth(0)
    }
}

use std::iter::Sum;
use std::ops::Div;
fn average<T>(v: &[T]) -> T
where
    T: Copy + Div<Output = T> + From<u16> + Sum<T>,
{
    v.iter().copied().sum::<T>() / From::from(v.len() as _)
}

impl<
        'a,
        T: Sized
            + Copy
            + std::ops::AddAssign
            + std::ops::DivAssign
            + std::iter::Sum
            + std::convert::From<u16>
            + std::ops::Div
            + Div<Output = T>,
    > Iterator for ChannelColumnView<'a, T>
{
    type Item = T;
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.horizontal_pos + (self.image.vertical_rate as usize * n) as i16
            >= self.image.image.width as i16
        {
            return None;
        }
        self.horizontal_pos += (self.image.horizontal_rate as usize * (n + 1)) as i16;
        match self.image.method {
            ChannelSubsamplingMethod::Skip => {
                let index: usize = (self.horizontal_pos
                    + self.vertical_pos * self.image.image.width as i16)
                    as usize;
                let l: T = self.image.image[self.image.channel_index][index];
                return Some(l);
            }
            ChannelSubsamplingMethod::Average => {
                let mut acc: Vec<T> = vec![];
                for x in (std::ops::Range {
                    start: 0,
                    end: self.image.horizontal_rate,
                }) {
                    let clamped_x =
                        std::cmp::min(self.image.image.width - 1, x + self.horizontal_pos as u16);
                    for y in (std::ops::Range {
                        start: 0,
                        end: self.image.vertical_rate,
                    }) {
                        let clamped_y = std::cmp::min(
                            self.image.image.height - 1,
                            y + self.vertical_pos as u16,
                        );
                        let index: usize =
                            (clamped_x + clamped_y * self.image.image.width) as usize;
                        acc.push(self.image.image[self.image.channel_index][index]);
                    }
                }
                return Some(average(&acc));
            }
        }
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }
}

#[cfg(test)]
mod test {
    use super::{ChannelIndex, ChannelSubsamplingMethod, ChannelView, Image};

    #[test]
    fn no_subsampling_test() {
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            luma: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ]
            .to_vec(),
            chroma_blue: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
            chroma_red: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
        };

        let mut my_itr: ChannelView<f32> = ChannelView::from_image(
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
            chroma_blue: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ]
            .to_vec(),
            luma: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
            chroma_red: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
        };

        let mut my_itr: ChannelView<f32> = ChannelView::from_image(
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
            chroma_blue: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ]
            .to_vec(),
            luma: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
            chroma_red: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
        };

        let mut my_itr: ChannelView<f32> = ChannelView::from_image(
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
        let my_img: Image<f32> = Image {
            width: 4,
            height: 4,
            chroma_blue: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ]
            .to_vec(),
            luma: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
            chroma_red: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
        };

        let mut my_itr: ChannelView<f32> = ChannelView::from_image(
            &my_img,
            ChannelIndex::ChromaBlue,
            2,
            1,
            ChannelSubsamplingMethod::Average,
        );

        let val = my_itr
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
            chroma_blue: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ]
            .to_vec(),
            luma: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
            chroma_red: [
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]
            .to_vec(),
        };

        let mut my_itr: ChannelView<f32> = ChannelView::from_image(
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
