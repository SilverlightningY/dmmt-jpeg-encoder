use std::time::{Duration, Instant};

use dmmt_jpeg_encoder::cosine_transform::{
    simple::SimpleDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};
use dmmt_jpeg_encoder::image::{ChannelSubsamplingConfig, ChannelSubsamplingMethod, Image};

const IMAGE_WIDTH: u16 = 3840;
const IMAGE_HEIGHT: u16 = 2160;
const IMAGE_SIZE: usize = IMAGE_WIDTH as usize * IMAGE_HEIGHT as usize;

fn create_test_color_channel() -> Vec<f32> {
    (0..IMAGE_SIZE)
        .map(|index| {
            let x = index as u16 % IMAGE_WIDTH;
            let y = index as u16 / IMAGE_WIDTH;
            let value = (x + y * 8) % 256;
            value as f32 / 255_f32
        })
        .collect()
}

fn create_test_image() -> Image<f32> {
    let color_channel = create_test_color_channel();
    Image::new(
        IMAGE_WIDTH,
        IMAGE_HEIGHT,
        color_channel,
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

const NUMBER_OF_ROUNDS: u32 = 10;

fn main() {
    println!("Creating test image");
    let test_image = create_test_image();
    println!("Splitting test image into squares");
    let test_blocks = cut_image_into_blocks(&test_image);
    println!("Starting transformation");
    let mut durations: Vec<Duration> = Vec::new();

    for index in 0..NUMBER_OF_ROUNDS {
        let round = index + 1;
        println!("Starting round {}", round);
        let start = Instant::now();

        for block in &test_blocks {
            SimpleDiscrete8x8CosineTransformer::transform(block);
        }

        let duration = start.elapsed();
        println!(
            "Finished round {} after {} microseconds",
            round,
            duration.as_micros(),
        );
        durations.push(duration);
    }

    println!("Transformation done");

    let min_duration = durations.iter().min().unwrap();
    let max_duration = durations.iter().max().unwrap();
    let avg_duration = durations.iter().sum::<Duration>() / NUMBER_OF_ROUNDS;
    println!(
        "Min: {}, Max: {}, Average: {}",
        min_duration.as_micros(),
        max_duration.as_micros(),
        avg_duration.as_micros()
    );
}
