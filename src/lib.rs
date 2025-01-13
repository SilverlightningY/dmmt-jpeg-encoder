use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

pub use cli::CLIParser;
use error::Error;
use image::{
    reader::ppm::PPMImageReader,
    subsampling::ChromaSubsamplingPreset,
    writer::jpeg::{JpegImageWriter, JpegTransformationOptions},
    ImageReader, ImageWriter,
};
use threadpool::ThreadPool;

pub mod binary_stream;
mod cli;
mod color;
pub mod cosine_transform;
mod error;
pub mod huffman;
pub mod image;
mod logger;

pub type Result<T> = std::result::Result<T, error::Error>;

pub trait BitPattern {
    fn to_bytes(&self) -> Box<[u8]>;
    fn bit_len(&self) -> usize;
}

pub struct Arguments {
    input_file: PathBuf,
    output_file: PathBuf,
    bits_per_channel: u8,
    chroma_subsampling_preset: ChromaSubsamplingPreset,
    number_of_threads: usize,
}

fn open_input_file(file_path: &Path) -> Result<File> {
    File::open(file_path).map_err(|e| {
        Error::UnableToOpenInputFileForReading(file_path.to_str().unwrap().to_owned(), e)
    })
}

fn open_output_file(file_path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .map_err(|e| {
            Error::UnableToOpenOutputFileForWriting(file_path.to_str().unwrap().to_owned(), e)
        })
}

pub fn convert_ppm_to_jpeg(arguments: &Arguments) -> Result<()> {
    let input_file = open_input_file(&arguments.input_file)?;
    let output_file = open_output_file(&arguments.output_file)?;
    let threadpool = ThreadPool::new(arguments.number_of_threads);

    let input_file_reader = BufReader::new(input_file);
    let mut image_reader = PPMImageReader::new(input_file_reader);
    let image = image_reader.read_image()?;

    let transformation_options = JpegTransformationOptions::from(arguments);
    let output_file_writer = BufWriter::new(output_file);
    let mut image_writer = JpegImageWriter::new(
        output_file_writer,
        &image,
        &transformation_options,
        &threadpool,
    );
    image_writer.write_image()
}
