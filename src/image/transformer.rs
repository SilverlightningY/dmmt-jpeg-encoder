use super::{Image, OutputImage, TransformationOptions};

pub struct JpegTransformer<'a, T> {
    image: &'a Image<T>,
}

impl<'a, T> JpegTransformer<'a, T> {
    fn new(image: &'a Image<T>) -> Self {
        JpegTransformer { image }
    }

    pub fn transform(&self, options: &TransformationOptions) -> OutputImage {
        OutputImage {
            width: self.image.width,
            height: self.image.height,
            chroma_subsampling_preset: options.chroma_subsampling_preset,
            bits_per_channel: options.bits_per_channel,
            subsampling_method: options.subsampling_method,
        }
    }
}
