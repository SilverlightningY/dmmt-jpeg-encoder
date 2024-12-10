use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use dmmt_jpeg_encoder::cosine_transform::{
    separated::SeparatedDiscrete8x8CosineTransformer,
    simple::SimpleDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};
use dmmt_jpeg_encoder::image::{ChannelSubsamplingConfig, ChannelSubsamplingMethod, Image};

const IMAGE_WIDTH: u16 = 3840;
const IMAGE_HEIGHT: u16 = 2160;
const IMAGE_SIZE: usize = IMAGE_WIDTH as usize * IMAGE_HEIGHT as usize;

struct Measurement {
    durations: Vec<Duration>,
    number_of_rounds: usize,
}

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
        .map(|square| -> [f32; 64] { square.try_into().unwrap() })
        .collect()
}

fn calculate_std_deviation_in_micros(mean: &Duration, measurements: &[Duration]) -> u64 {
    let mean_micros = mean.as_micros() as i128;
    let sum = measurements
        .iter()
        .map(|m| m.as_micros() as i128 - mean_micros)
        .map(|v| v.pow(2).unsigned_abs())
        .sum::<u128>();
    let variance = sum / measurements.len() as u128;
    (variance as f64).sqrt().round() as u64
}

fn transform_image(
    image: &Image<f32>,
    transformer: &impl Discrete8x8CosineTransformer,
) -> Duration {
    let mut blocks = cut_image_into_blocks(image);
    let start = Instant::now();
    for block in &mut blocks {
        transformer.transform(block);
    }
    start.elapsed()
}

fn measure_image_transformation_n_times(
    image: &Image<f32>,
    n: usize,
    transformer: &impl Discrete8x8CosineTransformer,
) -> Measurement {
    let mut durations: Vec<Duration> = Vec::new();

    let mut stdout = stdout();
    println!("Starting measurement");
    for round in 1..=n {
        print!("\rRound {}/{}", round, n);
        stdout.flush().unwrap();
        let duration = transform_image(image, transformer);
        durations.push(duration);
    }
    println!("\rMeasurement done");
    Measurement {
        durations,
        number_of_rounds: n,
    }
}

fn print_statistics(measurement: &Measurement) {
    let durations = &measurement.durations;
    let rounds = measurement.number_of_rounds as u32;
    let min_duration = durations.iter().min().unwrap();
    let max_duration = durations.iter().max().unwrap();
    let avg_duration = durations.iter().sum::<Duration>() / rounds;
    let std_deviation = calculate_std_deviation_in_micros(&avg_duration, durations);

    println!(
        "Rounds: {}, Min: {}, Max: {}, Average: {}, Std Deviation: {}",
        rounds,
        min_duration.as_micros(),
        max_duration.as_micros(),
        avg_duration.as_micros(),
        std_deviation,
    );
}

const NUMBER_OF_ROUNDS: u32 = 10;

fn main() {
    println!("Creating test image");
    let test_image = create_test_image();

    println!("Simple Algorithm");
    let measurement = measure_image_transformation_n_times(
        &test_image,
        NUMBER_OF_ROUNDS as usize,
        &SimpleDiscrete8x8CosineTransformer,
    );
    print_statistics(&measurement);

    // println!("Separated Algorithm");
    // println!("Arai Algorithm");
}

