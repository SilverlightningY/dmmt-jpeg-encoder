use std::env::args_os;
use std::ffi::OsString;
use std::io::{stdout, Result, Write};
use std::thread;
use std::time::{Duration, Instant};

use clap::builder::PossibleValue;
use clap::{arg, value_parser, Arg, ArgMatches, Command, ValueEnum};
use dmmt_jpeg_encoder::cosine_transform::{
    arai::AraiDiscrete8x8CosineTransformer, separated::SeparatedDiscrete8x8CosineTransformer,
    simple::SimpleDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};
use dmmt_jpeg_encoder::image::{ChannelSubsamplingConfig, ChannelSubsamplingMethod, Image};
use threadpool::ThreadPool;

const IMAGE_WIDTH: u16 = 3840;
const IMAGE_HEIGHT: u16 = 2160;
const IMAGE_SIZE: usize = IMAGE_WIDTH as usize * IMAGE_HEIGHT as usize;

#[derive(Debug, Clone)]
enum DCTAlgorithm {
    Simple,
    Separated,
    Arai,
}

impl ValueEnum for DCTAlgorithm {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Simple, Self::Separated, Self::Arai]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::Simple => Some(PossibleValue::new("Simple")),
            Self::Separated => Some(PossibleValue::new("Separated")),
            Self::Arai => Some(PossibleValue::new("Arai")),
        }
    }
}

#[derive(Debug)]
struct CLIParser {
    command: Command,
}

impl CLIParser {
    fn new() -> Self {
        let command = Self::create_base_command();
        let command = Self::register_arguments(command);
        Self { command }
    }

    fn parse<I, T>(&mut self, itr: I) -> Arguments
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = self
            .command
            .try_get_matches_from_mut(itr)
            .unwrap_or_else(|e| e.exit());
        Self::extract_arguments(&matches)
    }

    fn create_base_command() -> Command {
        Command::new("dct_timing")
    }

    fn register_arguments(command: Command) -> Command {
        let command = Self::register_threads_argument(command);
        let command = Self::register_algorithm_argument(command);
        Self::register_rounds_argument(command)
    }

    fn register_threads_argument(command: Command) -> Command {
        command.arg(Self::create_threads_argument())
    }

    fn register_rounds_argument(command: Command) -> Command {
        command.arg(Self::create_rounds_argument())
    }

    fn register_algorithm_argument(command: Command) -> Command {
        command.arg(Self::crate_algorithm_argument())
    }

    fn create_rounds_argument() -> Arg {
        arg!(-r --rounds <ROUNDS> "Number of Rounds")
            .default_value("1000")
            .required(false)
            .value_parser(value_parser!(usize))
    }

    fn create_threads_argument() -> Arg {
        arg!(-t --threads <THREADS> "Number of Threads")
            .default_value(get_number_of_threads().unwrap_or(1).to_string())
            .required(false)
            .value_parser(value_parser!(usize))
    }

    fn crate_algorithm_argument() -> Arg {
        arg!(-a --algorithm <ALGO> "DCT Algorithm")
            .default_value("Arai")
            .value_parser(value_parser!(DCTAlgorithm))
    }

    fn extract_arguments(matches: &ArgMatches) -> Arguments {
        Arguments {
            rounds: Self::extract_rounds_argument(matches),
            threads: Self::extract_threads_argument(matches),
            algorithm: Self::extract_algorithm_argument(matches),
        }
    }

    fn extract_rounds_argument(matches: &ArgMatches) -> usize {
        matches
            .get_one::<usize>("rounds")
            .expect("Required argument rounds not privided")
            .to_owned()
    }

    fn extract_threads_argument(matches: &ArgMatches) -> usize {
        matches
            .get_one::<usize>("threads")
            .expect("Required argument threads not provided")
            .to_owned()
    }

    fn extract_algorithm_argument(matches: &ArgMatches) -> DCTAlgorithm {
        matches
            .get_one::<DCTAlgorithm>("algorithm")
            .expect("Required argument algorithm not provided")
            .to_owned()
    }
}

struct Arguments {
    rounds: usize,
    threads: usize,
    algorithm: DCTAlgorithm,
}

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
        transformer.transform_on_threadpool(threadpool, channel_ptr, channel.len(), 700);
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
    let mut cli_parser = CLIParser::new();
    let arguments = cli_parser.parse(args_os());
    let number_of_rounds = arguments.rounds;
    let number_of_threads = arguments.threads;

    println!("Creating test image");
    let test_image = create_test_image();
    let blocks = cut_image_into_blocks(&test_image);
    let channel = blocks.into_flattened();
    println!("Creating Threadpool with {} threads", number_of_threads);
    let threadpool = ThreadPool::new(number_of_threads);

    match arguments.algorithm {
        DCTAlgorithm::Simple => {
            run_simple_algorithm_measurement(&channel, number_of_rounds, &threadpool);
        }
        DCTAlgorithm::Separated => {
            run_separated_algorithm_measurement(&channel, number_of_rounds, &threadpool);
        }
        DCTAlgorithm::Arai => {
            run_arai_algorithm_measurement(&channel, number_of_rounds, &threadpool);
        }
    }
}
