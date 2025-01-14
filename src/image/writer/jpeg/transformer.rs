use categorize::CategorizedBlock;
use frequency_block::FrequencyBlock;
use quantizer::Quantizer;
use symbol_counting::HuffmanCount;
use threadpool::ThreadPool;

use super::{Image, JpegTransformationOptions, OutputImage};
use crate::{
    color::YCbCrColorFormat,
    cosine_transform::{arai::AraiDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer},
    image::{
        subsampling::{Subsampler, SubsamplingConfig, SubsamplingMethod},
        ColorChannel,
    },
    Result,
};

pub mod categorize;
pub mod frequency_block;
pub mod quantizer;
mod symbol_counting;

pub struct CombinedColorChannels<T> {
    pub luma: T,
    pub chroma_red: T,
    pub chroma_blue: T,
}

type SeparateColorChannels<T> = CombinedColorChannels<ColorChannel<T>>;

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

    fn split_into_color_channels(
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

    fn apply_cosine_transform_on_all_channels_in_place(
        &self,
        channels: &mut SeparateColorChannels<f32>,
    ) {
        self.apply_cosine_transform_on_channel_in_place(&mut channels.luma);
        self.apply_cosine_transform_on_channel_in_place(&mut channels.chroma_red);
        self.apply_cosine_transform_on_channel_in_place(&mut channels.chroma_blue);
        self.threadpool.join();
    }

    fn apply_cosine_transform_on_channel_in_place(&self, channel: &mut ColorChannel<f32>) {
        let channel_length = channel.dots.len();
        let jobs_chunk_size = 700;
        unsafe {
            let channel_start = &raw mut channel.dots[0];
            AraiDiscrete8x8CosineTransformer.transform_on_threadpool(
                self.threadpool,
                channel_start,
                channel_length,
                jobs_chunk_size,
            );
        }
    }

    fn quantize_all_channels<'b>(
        &self,
        channels: &'b SeparateColorChannels<f32>,
    ) -> CombinedColorChannels<impl Iterator<Item = FrequencyBlock<i16>> + use<'b>> {
        let luma_quantizer = Quantizer::new(&channels.luma);
        let luma = luma_quantizer.quantize_channel();
        let chroma_red_quantizer = Quantizer::new(&channels.chroma_red);
        let chroma_red = chroma_red_quantizer.quantize_channel();
        let chroma_blue_quantizer = Quantizer::new(&channels.chroma_blue);
        let chroma_blue = chroma_blue_quantizer.quantize_channel();
        CombinedColorChannels {
            luma,
            chroma_red,
            chroma_blue,
        }
    }

    fn categorize_all_channels(
        &self,
        quantized_channels: CombinedColorChannels<impl Iterator<Item = FrequencyBlock<i16>>>,
    ) -> CombinedColorChannels<Vec<CategorizedBlock>> {
        let luma = categorize::categorize_channel(quantized_channels.luma);
        let chroma_red = categorize::categorize_channel(quantized_channels.chroma_red);
        let chroma_blue = categorize::categorize_channel(quantized_channels.chroma_blue);
        CombinedColorChannels {
            luma,
            chroma_red,
            chroma_blue,
        }
    }

    pub fn transform(&self) -> Result<OutputImage> {
        let color_dots = self.convert_color_format();
        let color_channels = self.split_into_color_channels(color_dots);
        let mut color_channels = self.subsample_all_channels(&color_channels);
        self.apply_cosine_transform_on_all_channels_in_place(&mut color_channels);
        let quantized_channels = self.quantize_all_channels(&color_channels);
        let categorized_channels = self.categorize_all_channels(quantized_channels);

        let luma_huffman_symbol_counts = HuffmanCount::from(&categorized_channels.luma);

        let chroma_huffman_symbol_counts = HuffmanCount::from_iter(
            categorized_channels
                .chroma_blue
                .iter()
                .chain(categorized_channels.chroma_red.iter()),
        );

        Ok(OutputImage {
            width: self.image.width,
            height: self.image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            luma_ac_huffman: luma_huffman_symbol_counts.generate_ac_huffman_code(),
            luma_dc_huffman: luma_huffman_symbol_counts.generate_dc_huffman_code(),
            chroma_ac_huffman: chroma_huffman_symbol_counts.generate_ac_huffman_code(),
            chroma_dc_huffman: chroma_huffman_symbol_counts.generate_dc_huffman_code(),
            blockwise_image_data: categorized_channels,
        })
    }
}
