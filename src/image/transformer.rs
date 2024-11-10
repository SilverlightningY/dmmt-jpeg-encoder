use super::{Image, OutputImage, TransformationOptions};
use crate::Result;

pub struct JpegTransformer<'a, T> {
    image: &'a Image<T>,
}

impl<'a, T> JpegTransformer<'a, T> {
    pub fn new(image: &'a Image<T>) -> Self {
        JpegTransformer { image }
    }

    pub fn transform(&self, options: &TransformationOptions) -> Result<OutputImage> {
        Ok(OutputImage {
            width: self.image.width,
            height: self.image.height,
            chroma_subsampling_preset: options.chroma_subsampling_preset,
            bits_per_channel: options.bits_per_channel,
            subsampling_method: options.chroma_subsampling_method,
        })
    }
}
