use std::path::PathBuf;

pub use cli::CLIParser;
use image::{ChannelSubsamplingMethod, ChromaSubsamplingPreset};
use std::error::Error;

pub mod binary_stream;
mod cli;
mod color;
mod image;

pub struct Arguments {
    input_file: PathBuf,
    output_file: PathBuf,
    bits_per_channel: u8,
    chroma_subsampling_preset: ChromaSubsamplingPreset,
    chroma_subsampling_method: ChannelSubsamplingMethod,
}

pub fn convert_ppm_to_jpeg(arguments: &Arguments) -> Result<(), Box<dyn Error>> {
    Ok(())
}
