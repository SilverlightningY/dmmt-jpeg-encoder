use std::io::{stdout, Write};
use std::time::{Duration, Instant};

use dmmt_jpeg_encoder::cosine_transform::{
    // arai::AraiDiscrete8x8CosineTransformer, separated::SeparatedDiscrete8x8CosineTransformer,
    simple::SimpleDiscrete8x8CosineTransformer,
    Discrete8x8CosineTransformer,
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

fn transform_channel(
    channel: &mut [f32],
    transformer: &impl Discrete8x8CosineTransformer,
) -> Duration {
    let start = Instant::now();
    for block_start_index in (0..channel.len()).step_by(64) {
        unsafe {
            transformer.transform(&raw mut channel[block_start_index]);
        }
    }
    start.elapsed()
}

fn measure_image_transformation_n_times(
    channel: &mut [f32],
    n: usize,
    transformer: &impl Discrete8x8CosineTransformer,
) -> Measurement {
    let mut durations: Vec<Duration> = Vec::new();

    let mut stdout = stdout();
    println!("Starting measurement");
    for round in 1..=n {
        print!("\rRound {}/{}", round, n);
        stdout.flush().unwrap();
        let mut channel = Vec::from_iter(channel.iter().copied());
        let duration = transform_channel(&mut channel, transformer);
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

fn run_simple_algorithm_measurement(channel: &mut [f32], rounds: usize) {
    println!("Simple Algorithm");
    let measurement =
        measure_image_transformation_n_times(channel, rounds, &SimpleDiscrete8x8CosineTransformer);
    print_statistics(&measurement);
}

// fn run_separated_algorithm_measurement(image: &Image<f32>, rounds: usize) {
//     println!("Separated Algorithm");
//     let measurement =
//         measure_image_transformation_n_times(image, rounds, &SeparatedDiscrete8x8CosineTransformer);
//     print_statistics(&measurement);
// }
//
// fn run_arai_algorithm_measurement(image: &Image<f32>, rounds: usize) {
//     println!("Arai Algorithm");
//     let measurement =
//         measure_image_transformation_n_times(image, rounds, &AraiDiscrete8x8CosineTransformer);
//     print_statistics(&measurement);
// }
//
fn main() {
    println!("Creating test image");
    let test_image = create_test_image();
    let blocks = cut_image_into_blocks(&test_image);
    let mut channel = blocks.into_flattened();

    run_simple_algorithm_measurement(&mut channel, 5);
    // run_separated_algorithm_measurement(&test_image, 140);
    // run_arai_algorithm_measurement(&test_image, 160);
}
