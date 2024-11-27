use super::{Image, OutputImage, TransformationOptions};
use crate::Result;

pub struct JpegTransformer<'a> {
    options: &'a TransformationOptions,
}

impl<'a> JpegTransformer<'a> {
    pub fn new(options: &'a TransformationOptions) -> Self {
        JpegTransformer { options }
    }

    pub fn transform<T>(&self, image: &Image<T>) -> Result<OutputImage> {
        Ok(OutputImage {
            width: image.width,
            height: image.height,
            chroma_subsampling_preset: self.options.chroma_subsampling_preset,
            bits_per_channel: self.options.bits_per_channel,
            subsampling_method: self.options.chroma_subsampling_method,
        })
    }
}
