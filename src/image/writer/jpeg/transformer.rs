use threadpool::ThreadPool;

use super::{Image, JpegTransformationOptions, OutputImage};
use crate::{
    color::YCbCrColorFormat,
    huffman::{
        code::HuffmanCodeGenerator, length_limited::LengthLimitedHuffmanCodeGenerator,
        SymbolCodeLength, SymbolFrequency,
    },
    image::{
        subsampling::{Subsampler, SubsamplingConfig, SubsamplingMethod},
        ColorChannel,
    },
    Result,
};

struct SeparateColorChannels<T> {
    luma: ColorChannel<T>,
    chroma_red: ColorChannel<T>,
    chroma_blue: ColorChannel<T>,
}

pub struct Transformer<'a> {
    options: &'a JpegTransformationOptions,
    image: &'a Image<f32>,
    threadpool: &'a ThreadPool,
}

impl<'a> Transformer<'a> {
    pub fn new(
        image: &'a Image<f32>,
        options: &'a JpegTransformationOptions,
        threadpool: &'a ThreadPool,
    ) -> Self {
        Transformer {
            options,
            image,
            threadpool,
        }
    }

    fn convert_color_format(&self) -> impl Iterator<Item = YCbCrColorFormat<f32>> + use<'a> {
        self.image.dots.iter().map(YCbCrColorFormat::from)
    }

    fn split_color_channels(
        &self,
        dots: impl Iterator<Item = YCbCrColorFormat<f32>>,
    ) -> SeparateColorChannels<f32> {
        let capacity = self.image.dots.len();
        let mut luma_dots = Vec::with_capacity(capacity);
        let mut chroma_red_dots = Vec::with_capacity(capacity);
        let mut chroma_blue_dots = Vec::with_capacity(capacity);
        for dot in dots {
            luma_dots.push(dot.luma);
            chroma_red_dots.push(dot.chroma_red);
            chroma_blue_dots.push(dot.chroma_blue);
        }
        let width = self.image.width;
        let height = self.image.height;
        SeparateColorChannels {
            luma: ColorChannel::new(width, height, luma_dots),
            chroma_red: ColorChannel::new(width, height, chroma_red_dots),
            chroma_blue: ColorChannel::new(width, height, chroma_blue_dots),
        }
    }

    fn subsample_color_channel(&self, color_channel: &ColorChannel<f32>) -> Vec<f32> {
        let config: SubsamplingConfig = self.options.chroma_subsampling_preset.into();
        let subsampler = Subsampler::new(color_channel, &config);
        subsampler.subsample_to_square_structure(8)
    }

    fn subsample_luma_channel(&self, luma_channel: &ColorChannel<f32>) -> Vec<f32> {
        let config = SubsamplingConfig {
            horizontal_rate: 1,
            vertical_rate: 1,
            method: SubsamplingMethod::Skip,
        };
        let subsampler = Subsampler::new(luma_channel, &config);
        subsampler.subsample_to_square_structure(8)
    }

    fn subsample_all_channels(
        &self,
        channels: &SeparateColorChannels<f32>,
    ) -> SeparateColorChannels<f32> {
        let luma = ColorChannel {
            dots: self.subsample_luma_channel(&channels.luma),
            ..channels.luma
        };
        let chroma_red = ColorChannel {
            dots: self.subsample_color_channel(&channels.chroma_red),
            ..channels.chroma_red
        };
        let chroma_blue = ColorChannel {
            dots: self.subsample_color_channel(&channels.chroma_blue),
            ..channels.chroma_blue
        };
        SeparateColorChannels {
            luma,
            chroma_red,
            chroma_blue,
        }
    }

    fn generate_code_lengths(symfreqs: &[SymbolFrequency]) -> Vec<SymbolCodeLength> {
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(15);
        let mut symlens = generator.generate_with_symbols(symfreqs);
        symlens[0].length += 1;
        symlens
    }

    pub fn transform(&self) -> Result<OutputImage> {
        let color_dots = self.convert_color_format();
        let color_channels = self.split_color_channels(color_dots);
        let color_channels = self.subsample_all_channels(&color_channels);
        #[rustfmt::skip]
        let mut ac_dummy = [(1, 14), (2, 30), (3, 4), (4, 7), (5, 9), (6, 4), (7, 42), (8, 1),
            (9, 14), (10, 5), (11, 14), (12, 30), (13, 4), (14, 7), (15, 9), (16, 4), (17, 42),
            (18, 1), (19, 14), (20, 5), (21, 14), (22, 30), (23, 4), (24, 7), (25, 9), (26, 4),
            (27, 42), (28, 1), (29, 14), (30, 12), (31, 32), (32, 1)]
            .map(SymbolFrequency::from);
        ac_dummy.sort_by_key(|f| f.frequency);

        Ok(OutputImage {
            width: self.image.width,
            height: self.image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            luma_ac_huffman: Self::generate_code_lengths(&ac_dummy),
            luma_dc_huffman: Self::generate_code_lengths(&ac_dummy),
            chroma_ac_huffman: Self::generate_code_lengths(&ac_dummy),
            chroma_dc_huffman: Self::generate_code_lengths(&ac_dummy),
        })
    }
}
