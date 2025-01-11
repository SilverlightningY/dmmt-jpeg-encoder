use categorize::CategorizedBlock;
use frequency_block::FrequencyBlock;
use quantizer::Quantizer;
use threadpool::ThreadPool;

use super::{Image, JpegTransformationOptions, OutputImage};
use crate::{
    color::YCbCrColorFormat,
    cosine_transform::{arai::AraiDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer},
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

pub mod categorize;
mod frequency_block;
mod quantizer;

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

struct HuffmanCount {
    ac_count: Vec<SymbolFrequency>,
    dc_count: Vec<SymbolFrequency>,
}

impl HuffmanCount {
    pub fn new() -> Self {
        Self {
            ac_count: Vec::new(),
            dc_count: Vec::new(),
        }
    }

    pub fn sort_counts_by_frequencies(&mut self) {
        self.ac_count.sort_by_key(|s| s.frequency);
        self.dc_count.sort_by_key(|s| s.frequency);
    }
}

fn ac_components_into_occurences<'a, T: Iterator<Item = &'a categorize::LeadingZerosToken>>(
    ac_components: T,
    ac_occurences: &mut [usize; 256],
) {
    for ac_token in ac_components {
        let ac_token_encodable_part = ac_token.combined_symbol();
        ac_occurences[ac_token_encodable_part as usize] += 1;
    }
}

fn channel_symbols_into_occurences(
    channel: &Vec<CategorizedBlock>,
    dc_occurences: &mut [usize; 16],
    ac_occurences: &mut [usize; 256],
) {
    for block in channel {
        let dc_encodable_part = block.dc_category.pattern_length;
        dc_occurences[dc_encodable_part as usize] += 1;
        ac_components_into_occurences(block.ac_tokens.iter(), ac_occurences);
    }
}

fn channels_into_occurences<'a, T: Iterator<Item = &'a Vec<CategorizedBlock>>>(
    channels: T,
    dc_occurences: &mut [usize; 16],
    ac_occurences: &mut [usize; 256],
) {
    for channel in channels {
        channel_symbols_into_occurences(channel, dc_occurences, ac_occurences);
    }
}

fn counts_into_symbol_frequencies_vec(out: &mut Vec<SymbolFrequency>, occurences: &[usize]) {
    for (i, &occurence) in occurences.iter().enumerate() {
        if occurence != 0 {
            out.push(SymbolFrequency::new(i as u8, occurence));
        }
    }
}

fn get_huffman_encodable_symbols_and_frequencies_from_channels<
    'a,
    T: Iterator<Item = &'a Vec<CategorizedBlock>>,
>(
    channels: T,
) -> HuffmanCount {
    let mut counts = HuffmanCount::new();

    let mut dc_occurences: [usize; 16] = [0; 16];
    let mut ac_occurences: [usize; 256] = [0; 256];

    channels_into_occurences(channels, &mut dc_occurences, &mut ac_occurences);

    counts_into_symbol_frequencies_vec(&mut counts.dc_count, &dc_occurences);
    counts_into_symbol_frequencies_vec(&mut counts.ac_count, &ac_occurences);

    counts.sort_counts_by_frequencies();

    counts
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

    fn generate_code_lengths(symfreqs: &[SymbolFrequency]) -> Vec<SymbolCodeLength> {
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(15);
        let mut symlens = generator.generate_with_symbols(symfreqs);
        symlens[0].length += 1;
        symlens
    }

    pub fn transform(&self) -> Result<OutputImage> {
        let color_dots = self.convert_color_format();
        let color_channels = self.split_into_color_channels(color_dots);
        let mut color_channels = self.subsample_all_channels(&color_channels);
        self.apply_cosine_transform_on_all_channels_in_place(&mut color_channels);
        let quantized_channels = self.quantize_all_channels(&color_channels);
        let categorized_channels = self.categorize_all_channels(quantized_channels);

        let luma_huffman_symbol_counts =
            get_huffman_encodable_symbols_and_frequencies_from_channels(
                [&categorized_channels.luma].into_iter(),
            );

        let chroma_huffman_symbol_counts =
            get_huffman_encodable_symbols_and_frequencies_from_channels(
                [
                    &categorized_channels.chroma_blue,
                    &categorized_channels.chroma_red,
                ]
                .into_iter(),
            );

        Ok(OutputImage {
            width: self.image.width,
            height: self.image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            luma_ac_huffman: Self::generate_code_lengths(&luma_huffman_symbol_counts.ac_count),
            luma_dc_huffman: Self::generate_code_lengths(&luma_huffman_symbol_counts.dc_count),
            chroma_ac_huffman: Self::generate_code_lengths(&chroma_huffman_symbol_counts.ac_count),
            chroma_dc_huffman: Self::generate_code_lengths(&chroma_huffman_symbol_counts.dc_count),
	    blockwise_image_data: categorized_channels
        })
    }
}

#[cfg(test)]
mod test {
    use crate::huffman::SymbolFrequency;

    use super::{
        categorize::{CategorizedBlock, CategoryEncodedInteger, LeadingZerosToken},
        get_huffman_encodable_symbols_and_frequencies_from_channels, HuffmanCount,
    };

    #[test]
    fn count_symbols_and_frequencies_test() {
        let test_blocks_channel_1: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(30), // DC symbol: 5
                vec![
                    LeadingZerosToken::new(0, 300), // AC symbol: 0b00001001 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(4, 5),   // AC symbol: 0b01000011 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(0), // DC symbol: 0
                vec![
                    LeadingZerosToken::new(0, 600), // AC symbol: 0b00001010 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(4, 15),  // AC symbol: 0b01000100 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];
        let test_blocks_channel_2: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(60), // DC symbol: 6
                vec![
                    LeadingZerosToken::new(0, 100), // AC symbol: 0b00000111 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(2, 7),   // AC symbol: 0b00100011 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(1), // DC symbol: 1
                vec![
                    LeadingZerosToken::new(0, 900), // AC symbol: 0b00001010 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(0, 1),   // AC symbol: 0b00000001 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];

        let expected: HuffmanCount = HuffmanCount {
            dc_count: vec![
                SymbolFrequency::new(5, 1),
                SymbolFrequency::new(0, 1),
                SymbolFrequency::new(6, 1),
                SymbolFrequency::new(1, 1),
            ],
            ac_count: vec![
                SymbolFrequency::new(0b00001001, 1),
                SymbolFrequency::new(0b11110000, 4),
                SymbolFrequency::new(0b01000011, 1),
                SymbolFrequency::new(0b00000000, 4),
                SymbolFrequency::new(0b00001010, 2),
                SymbolFrequency::new(0b01000100, 1),
                SymbolFrequency::new(0b00000111, 1),
                SymbolFrequency::new(0b00100011, 1),
                SymbolFrequency::new(0b00000001, 1),
            ],
        };

        let got = get_huffman_encodable_symbols_and_frequencies_from_channels(
            [test_blocks_channel_1, test_blocks_channel_2].iter(),
        );

        for symfreq in got.dc_count.iter() {
            let mut found = false;
            for comp in expected.dc_count.iter() {
                if symfreq.symbol == comp.symbol {
                    assert_eq!(symfreq.frequency, comp.frequency);
                    found = true;
                }
            }
            assert!(found);
        }
        for symfreq in got.ac_count.iter() {
            let mut found = false;
            for comp in expected.ac_count.iter() {
                if symfreq.symbol == comp.symbol {
                    assert_eq!(symfreq.frequency, comp.frequency);
                    found = true;
                }
            }
            assert!(found);
        }
    }
}
