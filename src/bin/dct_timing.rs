use std::time::Instant;

use dmmt_jpeg_encoder::cosine_transform::{
    simple::SimpleDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};
use dmmt_jpeg_encoder::image::{ChannelSubsamplingConfig, ChannelSubsamplingMethod, Image};

const IMAGE_WIDTH: u16 = 3840;
const IMAGE_HEIGHT: u16 = 2160;
const IMAGE_SIZE: usize = IMAGE_WIDTH as usize * IMAGE_HEIGHT as usize;

fn create_test_color_channel() -> [f32; IMAGE_SIZE] {
    (0..IMAGE_SIZE)
        .map(|index| {
            let x = index as u16 % IMAGE_WIDTH;
            let y = index as u16 / IMAGE_WIDTH;
            let value = (x + y * 8) % 256;
            let scaled_value = value as f32 / 255_f32;
            (index, scaled_value)
        })
        .fold([f32::default(); IMAGE_SIZE], |mut acc, (index, value)| {
            acc[index] = value;
            acc
        })
}

fn create_test_image() -> Image<f32> {
    let color_channel = create_test_color_channel();
    Image::new(
        IMAGE_WIDTH,
        IMAGE_HEIGHT,
        color_channel.to_vec(),
        Vec::new(),
        Vec::new(),
    )
}

fn cut_image_into_blocks(image: &Image<f32>) -> Vec<[f32; 64]> {
    let subsampling_config = ChannelSubsamplingConfig {
        vertical_rate: 1,
        horizontal_rate: 1,
        method: ChannelSubsamplingMethod::Skip,
    };
    image
        .luma_channel()
        .subsampling_iter(&subsampling_config)
        .into_square_iter(8)
        .fold(Vec::new(), |mut acc, square| {
            let block: [f32; 64] = square[0..64].try_into().unwrap();
            acc.push(block);
            acc
        })
}

fn main() {
    let test_blocks = cut_image_into_blocks(&create_test_image());
    let start = Instant::now();
    for _ in 0..1 {
        for block in &test_blocks {
            SimpleDiscrete8x8CosineTransformer::transform(block);
        }
    }
    let duration = start.elapsed();
    println!("Time elapsed: {}", duration.as_micros());
}
