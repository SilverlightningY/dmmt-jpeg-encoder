use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

pub use cli::CLIParser;
use error::Error;
use image::{
    encoder::Encoder,
    ppm_parser::{PPMParser, PPMTokenizer},
    transformer::JpegTransformer,
    ChannelSubsamplingMethod, ChromaSubsamplingPreset, TransformationOptions,
};

pub mod binary_stream;
mod cli;
mod color;
mod error;
mod image;

pub type Result<T> = std::result::Result<T, error::Error>;

pub struct Arguments {
    input_file: PathBuf,
    output_file: PathBuf,
    bits_per_channel: u8,
    chroma_subsampling_preset: ChromaSubsamplingPreset,
    chroma_subsampling_method: ChannelSubsamplingMethod,
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
    let image = PPMParser::parse(PPMTokenizer::new(BufReader::new(&input_file)))?;
    let transformer = JpegTransformer::new(&image);
    let transformation_options = TransformationOptions::from(arguments);
    let output_image = transformer.transform(&transformation_options)?;
    let mut output_file_writer = BufWriter::new(&output_file);
    let mut encoder = Encoder::new(&output_image, &mut output_file_writer);
    encoder.encode()?;
    Ok(())
}
