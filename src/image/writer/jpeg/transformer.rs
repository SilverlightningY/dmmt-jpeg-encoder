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

mod frequency_block;
mod quantizer;

struct CombinedColorChannels<T> {
    luma: T,
    chroma_red: T,
    chroma_blue: T,
}

type SeparateColorChannels<T> = CombinedColorChannels<ColorChannel<T>>;

pub struct Transformer<'a> {
    options: &'a JpegTransformationOptions,
    image: &'a Image<f32>,
    threadpool: &'a ThreadPool,
}

#[derive(Clone, Copy)]
struct CategoryEncodedInteger {
    category: u8,
    pattern: u16,
}

impl From<i16> for CategoryEncodedInteger {
    fn from(value: i16) -> Self {
        // which category?
        let mut cat = 0;
        if value != 0 {
            for c in 0..15 {
                if value.abs() < (1 << c) {
                    cat = c;
                    break;
                }
            }
        } else {
            return CategoryEncodedInteger {
                category: 0,
                pattern: 0,
            };
        }
        // which bit pattern?
        let mut pattern = value.abs() - (1 << (cat - 1));
        if value > 0 {
            pattern += 1 << (cat - 1);
        } else {
            pattern = ((1 << (cat - 1)) - 1) - pattern;
        }
        // left-align bit pattern
        pattern <<= 16 - cat;
        CategoryEncodedInteger {
            category: cat,
            pattern: pattern as u16,
        }
    }
}

pub struct ACToken {
    zeros_before: u8,
    symbol: CategoryEncodedInteger,
}

impl ACToken {
    fn new(zeros_before: u8, symbol: i16) -> Self {
        Self {
            zeros_before,
            symbol: CategoryEncodedInteger::from(symbol),
        }
    }

    fn get_huffman_encodable_part(&self) -> u8 {
        let mut result: u8 = 0;
        result |= self.zeros_before << 4;
        result |= self.symbol.category;
        result
    }

    fn get_pattern(&self) -> CategoryEncodedInteger {
        self.symbol
    }
}

pub fn categorize_ac_tokens<'a, T: Iterator<Item = &'a i16>>(sequence: T) -> Vec<ACToken> {
    let mut result: Vec<ACToken> = Vec::new();
    let mut zeros_encountered = 0;
    for &i in sequence {
        if i == 0 {
            zeros_encountered += 1;
        } else {
            while zeros_encountered > 15 {
                result.push(ACToken::new(zeros_encountered, 0));
                zeros_encountered -= 16;
            }
            result.push(ACToken::new(zeros_encountered, i));
            zeros_encountered = 0;
        }
    }
    if zeros_encountered != 0 {
        result.push(ACToken::new(0, 0));
    }
    result
}

struct CategorizedBlock {
    dc_difference: CategoryEncodedInteger,
    ac_components: Vec<ACToken>,
}

impl CategorizedBlock {
    pub fn new(dc_difference: CategoryEncodedInteger, ac_components: Vec<ACToken>) -> Self {
        Self {
            dc_difference,
            ac_components,
        }
    }
}

fn categorize_channel<T: Iterator<Item = FrequencyBlock<i16>>>(
    frequency_blocks: T,
) -> Vec<CategorizedBlock> {
    let mut categorized_blocks: Vec<CategorizedBlock> = Vec::new();
    let mut last_dc = 0;
    for frequency_block in frequency_blocks {
        let current_dc = *frequency_block.dc();
        let dc_difference_categorized = CategoryEncodedInteger::from(current_dc - last_dc);
        last_dc = current_dc;
        let ac_run_length_categorized =
            categorize_ac_tokens(frequency_block.iter_zig_zag().skip(1));
        categorized_blocks.push(CategorizedBlock::new(
            dc_difference_categorized,
            ac_run_length_categorized,
        ));
    }
    categorized_blocks
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

    // TODO: clean code, move to helper
    for channel in channels {
        for block in channel {
            let dc_encodable_part = block.dc_difference.category;
            dc_occurences[dc_encodable_part as usize] += 1;
            for ac_token in block.ac_components.iter() {
                let ac_token_encodable_part = ac_token.get_huffman_encodable_part();
                ac_occurences[ac_token_encodable_part as usize] += 1;
            }
        }
    }

    // TODO: clean code
    for i in 0..16 {
        let dc_occurence = dc_occurences[i];
        if dc_occurence != 0 {
            counts
                .dc_count
                .push(SymbolFrequency::new(i as u8, dc_occurence));
        }
    }

    for i in 0..256 {
        let ac_occurence = ac_occurences[i];
        if ac_occurence != 0 {
            counts
                .ac_count
                .push(SymbolFrequency::new(i as u8, ac_occurence));
        }
    }

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
        let luma = categorize_channel(quantized_channels.luma);
        let chroma_red = categorize_channel(quantized_channels.chroma_red);
        let chroma_blue = categorize_channel(quantized_channels.chroma_blue);
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

        // count symfreqs for huffman code generation

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

#[cfg(test)]
mod test {
    use crate::huffman::SymbolFrequency;

    use super::{categorize_ac_tokens, get_huffman_encodable_symbols_and_frequencies_from_channels, ACToken, CategorizedBlock, CategoryEncodedInteger, HuffmanCount};

    #[test]
    fn categorize_test() {
        let expected = [
            CategoryEncodedInteger {
                category: 6,
                pattern: 0b11100100_00000000u16,
            },
            CategoryEncodedInteger {
                category: 6,
                pattern: 0b10110100_00000000u16,
            },
            CategoryEncodedInteger {
                category: 1,
                pattern: 0b10000000_00000000u16,
            },
            CategoryEncodedInteger {
                category: 5,
                pattern: 0b00001000_00000000u16,
            },
        ];
        let input: Vec<i16> = vec![57, 45, 1, -30];
        for i in 0..4 {
            let v = input[i];
            let r = CategoryEncodedInteger::from(v);
            assert_eq!(expected[i].category, r.category);
            assert_eq!(expected[i].pattern, r.pattern);
        }
    }

    #[test]
    fn categorize_ac_tokens_test() {
        let test_sequence: Vec<i16> = vec![57, 45, 0, 0, 0, 0, 23, 0, -30, -16, 0, 0, 1, 0];
        let expect_sequence: Vec<ACToken> = vec![
            ACToken::new(0, 57),
            ACToken::new(0, 45),
            ACToken::new(4, 23),
            ACToken::new(1, -30),
            ACToken::new(0, -16),
            ACToken::new(2, 1),
            ACToken::new(0, 0),
        ];
        let got_sequence: Vec<ACToken> = categorize_ac_tokens(test_sequence.iter());

        for i in 0..got_sequence.len() {
            assert_eq!(
                got_sequence[i].zeros_before,
                expect_sequence[i].zeros_before
            );
            assert_eq!(
                got_sequence[i].symbol.category,
                expect_sequence[i].symbol.category
            );
            assert_eq!(
                got_sequence[i].symbol.pattern,
                expect_sequence[i].symbol.pattern
            );
        }
    }

    #[test]
    fn count_symbols_and_frequencies_test() {
        let test_blocks_channel_1: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(30), // DC symbol: 5
                vec![
                    ACToken::new(0, 300), // AC symbol: 0b00001001 x
                    ACToken::new(15, 0),  // AC symbol: 0b11110000 x
                    ACToken::new(4, 5),   // AC symbol: 0b01000011 x
                    ACToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(0), // DC symbol: 0
                vec![
                    ACToken::new(0, 600), // AC symbol: 0b00001010 x
                    ACToken::new(15, 0),  // AC symbol: 0b11110000 x
                    ACToken::new(4, 15),  // AC symbol: 0b01000100 x
                    ACToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];
	let test_blocks_channel_2: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(60), // DC symbol: 6
                vec![
                    ACToken::new(0, 100), // AC symbol: 0b00000111 x
                    ACToken::new(15, 0),  // AC symbol: 0b11110000 x
                    ACToken::new(2, 7),   // AC symbol: 0b00100011 x
                    ACToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(1), // DC symbol: 1
                vec![
                    ACToken::new(0, 900), // AC symbol: 0b00001010 x
                    ACToken::new(15, 0),  // AC symbol: 0b11110000 x
                    ACToken::new(0, 1),   // AC symbol: 0b00000001 x
                    ACToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];

	let expected: HuffmanCount = HuffmanCount {
	    dc_count: vec![SymbolFrequency::new(5,1),SymbolFrequency::new(0,1),SymbolFrequency::new(6,1),SymbolFrequency::new(1,1)],
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
	    ]
	};

	let got = get_huffman_encodable_symbols_and_frequencies_from_channels([test_blocks_channel_1, test_blocks_channel_2].iter());

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
