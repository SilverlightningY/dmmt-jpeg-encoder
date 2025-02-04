use std::io::Write;

mod encoder;
mod padder;
mod quantization_tables;
mod segment_marker_injector;
mod transformer;

use encoder::Encoder;
pub use quantization_tables::QuantizationTablePreset;
use threadpool::ThreadPool;
use transformer::{categorize::CategorizedBlock, CombinedColorChannels, Transformer};

use crate::{
    huffman::SymbolCodeLength,
    image::{subsampling::ChromaSubsamplingPreset, Image, ImageWriter},
    Arguments,
};

pub struct QuantizationTablePair<'a> {
    luma_table: &'a [u8; 64],
    chroma_table: &'a [u8; 64],
}

pub struct JpegTransformationOptions {
    pub chroma_subsampling_preset: ChromaSubsamplingPreset,
    pub bits_per_channel: u8,
    pub quantization_table_preset: QuantizationTablePreset,
}

impl From<&Arguments> for JpegTransformationOptions {
    fn from(value: &Arguments) -> Self {
        Self {
            chroma_subsampling_preset: value.chroma_subsampling_preset,
            bits_per_channel: value.bits_per_channel,
            quantization_table_preset: value.quantization_table_preset,
        }
    }
}

pub struct JpegImageWriter<'a, T: Write> {
    writer: T,
    image: &'a Image<f32>,
    options: &'a JpegTransformationOptions,
    threadpool: &'a ThreadPool,
}

impl<'a, T: Write> JpegImageWriter<'a, T> {
    pub fn new(
        writer: T,
        image: &'a Image<f32>,
        options: &'a JpegTransformationOptions,
        threadpool: &'a ThreadPool,
    ) -> Self {
        Self {
            writer,
            image,
            options,
            threadpool,
        }
    }
}

impl<T: Write> ImageWriter for JpegImageWriter<'_, T> {
    fn write_image(&mut self) -> crate::Result<()> {
        let transformer = Transformer::new(self.image, self.options, self.threadpool);
        let output_image = transformer.transform()?;
        let mut encoder = Encoder::new(&mut self.writer, &output_image);
        encoder.encode()?;
        self.writer
            .flush()
            .expect("Flushing of inner writer failed");
        Ok(())
    }
}

struct OutputImage {
    width: u16,
    height: u16,
    chroma_subsampling_preset: ChromaSubsamplingPreset,
    bits_per_channel: u8,
    luma_ac_huffman: Vec<SymbolCodeLength>,
    luma_dc_huffman: Vec<SymbolCodeLength>,
    chroma_ac_huffman: Vec<SymbolCodeLength>,
    chroma_dc_huffman: Vec<SymbolCodeLength>,
    blockwise_image_data: CombinedColorChannels<Vec<CategorizedBlock>>,
    quantization_table_pair: QuantizationTablePair<'static>,
}
