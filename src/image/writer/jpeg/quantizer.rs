use crate::{
    color::YCbCrColorFormat,
    image::{ColorChannel, Image},
};

#[rustfmt::skip]
const Q_TABLE: [f32;64] =  [
    16.0, 11.0, 10.0, 16.0, 24.0, 40.0, 51.0, 61.0, 
    12.0, 12.0, 14.0, 19.0, 26.0, 58.0, 60.0, 55.0, 
    14.0, 13.0, 16.0, 24.0, 40.0, 57.0, 69.0, 56.0,
    14.0, 17.0, 22.0, 29.0, 51.0, 87.0, 80.0, 62.0, 
    18.0, 22.0, 37.0, 56.0, 68.0, 109.0, 103.0, 77.0, 
    24.0, 35.0, 55.0, 64.0, 81.0, 104.0, 113.0, 92.0,
    49.0, 64.0, 78.0, 87.0, 103.0, 121.0, 120.0, 101.0,
    72.0, 92.0, 95.0, 98.0, 112.0, 100.0, 103.0, 99.0, 
];

pub struct Quantizer {
    image: Image<f32>,
    q_luma: Vec<Block>,
    q_cb: Vec<Block>,
    q_cr: Vec<Block>,
}
struct SeparateColorChannels<T> {
    luma: ColorChannel<T>,
    chroma_red: ColorChannel<T>,
    chroma_blue: ColorChannel<T>,
}

struct Block {
    values: Vec<f32>,
}

impl Block {
    fn new(values: Vec<f32>) -> Self {
        Block { values }
    }

    fn ac_zig(&self) -> f32 {
        // zigzag
        self.values[0]
    }
    fn dc(&self) -> f32 {
        self.values[0]
    }
    fn dc_diff(&self, block: Block) -> f32 {
        self.values[0] - block.values[0]
    }
}

impl Quantizer {
    pub fn new(image: Image<f32>) -> Self {
        let dots_len = image.dots.len();
        Quantizer {
            image,
            q_cr: Vec::with_capacity(dots_len / 64),
            q_cb: Vec::with_capacity(dots_len / 64),
            q_luma: Vec::with_capacity(dots_len / 64),
        }
    }

    pub fn convert_color_format(&self) -> impl Iterator<Item = YCbCrColorFormat<f32>> + use<'_> {
        self.image.dots.iter().map(YCbCrColorFormat::from)
    }

    fn split_into_color_channels(
        &self,
        dots: impl Iterator<Item = YCbCrColorFormat<f32>>,
    ) -> SeparateColorChannels<f32> {
        let capacity = self.image.dots.len();
        let mut luma_dots = Vec::with_capacity(capacity);
        let mut chroma_red_dots = Vec::with_capacity(capacity);
        let mut chroma_blue_dots = Vec::with_capacity(capacity);
        for dot in dots {
            luma_dots.push(dot.luma);
            chroma_red_dots.push(dot.chroma_red);
            chroma_blue_dots.push(dot.chroma_blue);
        }
        let width = self.image.width;
        let height = self.image.height;
        SeparateColorChannels {
            luma: ColorChannel::new(width, height, luma_dots),
            chroma_red: ColorChannel::new(width, height, chroma_red_dots),
            chroma_blue: ColorChannel::new(width, height, chroma_blue_dots),
        }
    }

    fn quantize_channel(&self, color_channel: ColorChannel<f32>) -> Vec<Block> {
        let mut vec: Vec<f32> = Vec::with_capacity(64);
        let mut block_list: Vec<Block> = Vec::new();
        for (index, dot) in color_channel.dots.iter().enumerate() {
            vec.push(((dot / Q_TABLE[index]) as i32) as f32);
            if vec.len() == 64 {
                block_list.push(Block::new(vec.clone()));
            }
        }
        block_list
    }

    fn quantize_channel_static(color_channel: ColorChannel<f32>) -> Vec<Block> {
        let mut vec: Vec<f32> = Vec::with_capacity(64);
        let mut block_list: Vec<Block> = Vec::new();
        for (index, dot) in color_channel.dots.iter().enumerate() {
            vec.push(dot / Q_TABLE[index]);
            if vec.len() == 64 {
                block_list.push(Block::new(vec.clone()));
            }
        }
        block_list
    }

    fn quantize_all_channels(&mut self, color_channels: SeparateColorChannels<f32>) {
        self.q_luma = self.quantize_channel(color_channels.luma);
        self.q_cb = self.quantize_channel(color_channels.chroma_blue);
        self.q_cr = self.quantize_channel(color_channels.chroma_red);
    }

    fn quantize(&mut self) {
        let color_dots = self.convert_color_format();
        let color_channels = self.split_into_color_channels(color_dots);
        self.quantize_all_channels(color_channels);
    }
}

#[cfg(test)]
mod tests {
    use crate::image::ColorChannel;

    use super::Quantizer;

    const IMAGE_SIZE: usize = 64;
    const IMAGE_WIDTH: u16 = 8;
    const IMAGE_HEIGHT: u16 = 8;

    fn create_test_color_channel() -> ColorChannel<f32> {
        let dots = (0..IMAGE_SIZE)
            .map(|index| {
                let x = index as u16 % IMAGE_WIDTH;
                let y = index as u16 / IMAGE_WIDTH;
                let value = (x + y * 8) % 256;
                value as f32 / 255_f32
            })
            .collect::<Vec<f32>>();
        ColorChannel::new(IMAGE_WIDTH, IMAGE_HEIGHT, dots)
    }

/*    #[test]
    fn test_general() {
        let test_channel: ColorChannel<f32> = create_test_color_channel();
        let res = Quantizer::quantize_channel_static(test_channel);
        println!("{:?}", res[0].values);
        assert_eq!(res[0].values[0], 1.0); // fail to print
    }*/
}
