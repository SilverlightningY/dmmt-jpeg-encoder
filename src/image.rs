use crate::color::RGBColorFormat;

pub mod reader;
pub mod subsampling;
pub mod writer;

pub struct Image<T> {
    width: u16,
    height: u16,
    dots: Vec<RGBColorFormat<T>>,
}

pub trait ImageReader<T> {
    fn read_image(&mut self) -> crate::Result<Image<T>>;
}

pub trait ImageWriter {
    fn write_image(&mut self) -> crate::Result<()>;
}

pub struct ColorChannel<T> {
    width: u16,
    height: u16,
    dots: Vec<T>,
}

impl<T> ColorChannel<T> {
    pub fn new(width: u16, height: u16, dots: Vec<T>) -> Self {
        Self {
            width,
            height,
            dots,
        }
    }
}
