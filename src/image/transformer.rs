use super::{Image, OutputImage, TransformationOptions};
use crate::{
    huffman::{
        code::HuffmanCodeGenerator, length_limited::LengthLimitedHuffmanCodeGenerator,
        SymbolCodeLength, SymbolFrequency,
    },
    Result,
};

pub struct JpegTransformer<'a> {
    options: &'a TransformationOptions,
}

impl<'a> JpegTransformer<'a> {
    pub fn new(options: &'a TransformationOptions) -> Self {
        JpegTransformer { options }
    }

    fn generate_code_lengths(symfreqs: &[SymbolFrequency]) -> Vec<SymbolCodeLength> {
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(15);
        let mut symlens = generator.generate_with_symbols(symfreqs);
        symlens[0].length += 1;
        symlens
    }

    pub fn transform<T>(&self, image: &Image<T>) -> Result<OutputImage> {
        #[rustfmt::skip]
        let mut ac_dummy = [(1, 14), (2, 30), (3, 4), (4, 7), (5, 9), (6, 4), (7, 42), (8, 1),
            (9, 14), (10, 5), (11, 14), (12, 30), (13, 4), (14, 7), (15, 9), (16, 4), (17, 42),
            (18, 1), (19, 14), (20, 5), (21, 14), (22, 30), (23, 4), (24, 7), (25, 9), (26, 4),
            (27, 42), (28, 1), (29, 14), (30, 12), (31, 32), (32, 1)]
            .map(SymbolFrequency::from);
        ac_dummy.sort_by_key(|f| f.frequency);

        Ok(OutputImage {
            width: image.width,
            height: image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            luma_ac_huffman: Self::generate_code_lengths(&ac_dummy),
            luma_dc_huffman: Self::generate_code_lengths(&ac_dummy),
            chroma_ac_huffman: Self::generate_code_lengths(&ac_dummy),
            chroma_dc_huffman: Self::generate_code_lengths(&ac_dummy),
        })
    }
}
