use std::io::{stdout, Result, Write};
use std::num::NonZero;
use std::thread;
use std::time::{Duration, Instant};

use dmmt_jpeg_encoder::cosine_transform::{
    arai::AraiDiscrete8x8CosineTransformer, separated::SeparatedDiscrete8x8CosineTransformer,
    simple::SimpleDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};
use dmmt_jpeg_encoder::image::{ChannelSubsamplingConfig, ChannelSubsamplingMethod, Image};
use threadpool::ThreadPool;

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
    transformer: &'static impl Discrete8x8CosineTransformer,
    threadpool: &ThreadPool,
) -> Duration {
    let start = Instant::now();
    unsafe {
        let channel_ptr = &raw mut channel[0];
        transformer.transform_on_threadpool(threadpool, channel_ptr, channel.len(), 200);
    }
    threadpool.join();
    start.elapsed()
}

fn measure_image_transformation_n_times(
    channel: &[f32],
    n: usize,
    transformer: &'static impl Discrete8x8CosineTransformer,
    threadpool: &ThreadPool,
) -> Measurement {
    let mut durations: Vec<Duration> = Vec::new();

    let mut stdout = stdout();
    println!("Starting measurement");
    for round in 1..=n {
        print!("\rRound {}/{}", round, n);
        stdout.flush().unwrap();
        let mut channel = Vec::from_iter(channel.iter().copied());
        let duration = transform_channel(&mut channel, transformer, threadpool);
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

fn run_simple_algorithm_measurement(channel: &[f32], rounds: usize, threadpool: &ThreadPool) {
    println!("Simple Algorithm");
    let measurement = measure_image_transformation_n_times(
        channel,
        rounds,
        &SimpleDiscrete8x8CosineTransformer,
        threadpool,
    );
    print_statistics(&measurement);
}

fn run_separated_algorithm_measurement(channel: &[f32], rounds: usize, threadpool: &ThreadPool) {
    println!("Separated Algorithm");
    let measurement = measure_image_transformation_n_times(
        channel,
        rounds,
        &SeparatedDiscrete8x8CosineTransformer,
        threadpool,
    );
    print_statistics(&measurement);
}

fn run_arai_algorithm_measurement(channel: &[f32], rounds: usize, threadpool: &ThreadPool) {
    println!("Arai Algorithm");
    let measurement = measure_image_transformation_n_times(
        channel,
        rounds,
        &AraiDiscrete8x8CosineTransformer,
        threadpool,
    );
    print_statistics(&measurement);
}

fn get_number_of_threads() -> Result<usize> {
    Ok(thread::available_parallelism()?.get())
}

fn main() {
    println!("Creating test image");
    let test_image = create_test_image();
    let blocks = cut_image_into_blocks(&test_image);
    let channel = blocks.into_flattened();
    let number_of_threads = get_number_of_threads().unwrap_or(4);
    let threadpool = ThreadPool::new(number_of_threads);

    run_simple_algorithm_measurement(&channel, 5, &threadpool);
    run_separated_algorithm_measurement(&channel, 140, &threadpool);
    run_arai_algorithm_measurement(&channel, 160, &threadpool);
}
