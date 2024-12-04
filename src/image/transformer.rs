use super::{Image, OutputImage, TransformationOptions};
use crate::{huffman::SymbolFrequency, Result};

pub struct JpegTransformer<'a> {
    options: &'a TransformationOptions,
}

impl<'a> JpegTransformer<'a> {
    pub fn new(options: &'a TransformationOptions) -> Self {
        JpegTransformer { options }
    }

    pub fn transform<T>(&self, image: &Image<T>) -> Result<OutputImage> {
        let syms_and_depths: Vec<SymbolFrequency> = vec![
            SymbolFrequency {
                symbol: 3,
                frequency: 2,
            },
            SymbolFrequency {
                symbol: 4,
                frequency: 3,
            },
            SymbolFrequency {
                symbol: 2,
                frequency: 4,
            },
            SymbolFrequency {
                symbol: 5,
                frequency: 4,
            },
            SymbolFrequency {
                symbol: 1,
                frequency: 4,
            },
        ];

        Ok(OutputImage {
            width: image.width,
            height: image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            subsampling_method: self.options.chroma_subsampling_method,
            luma_ac_huffman: syms_and_depths.clone(),
            luma_dc_huffman: syms_and_depths.clone(),
            chroma_ac_huffman: syms_and_depths.clone(),
            chroma_dc_huffman: syms_and_depths.clone(),
        })
    }
}
