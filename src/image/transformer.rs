use super::{Image, OutputImage, SymAndDepth, TransformationOptions};
use crate::Result;

pub struct JpegTransformer<'a> {
    options: &'a TransformationOptions,
}

impl<'a> JpegTransformer<'a> {
    pub fn new(options: &'a TransformationOptions) -> Self {
        JpegTransformer { options }
    }

    pub fn transform<T>(&self, image: &Image<T>) -> Result<OutputImage> {
        let syms_and_depths: Vec<SymAndDepth> = [(1, 4), (5, 4), (2, 4), (4, 3), (3, 2)].to_vec();

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
