use crate::{color::RGBColorFormat, image::Image};

pub struct PaddedImage {
    width: u16,
    height: u16,
    padded_width: u16,
    padded_height: u16,
    dots: Vec<RGBColorFormat<f32>>,
}

impl PaddedImage {
    pub fn new(image: Image<f32>, pad_nearest_width: u16, pad_nearest_height: u16) -> Self {
        let padded_width =
            ((image.width + pad_nearest_width - 1) / pad_nearest_width) * pad_nearest_width;
        let padded_height =
            ((image.height + pad_nearest_height - 1) / pad_nearest_height) * pad_nearest_height;

        let black_pixel: RGBColorFormat<f32> = RGBColorFormat::default();
        let mut dots = Vec::with_capacity(padded_height as usize * padded_width as usize);

        let mut position = 0;
        for _ in 0..image.height {
            for _ in 0..image.width {
                dots.push(image.dots[position]);
                position += 1;
            }
            for _ in image.width..padded_width {
                dots.push(black_pixel.clone());
            }
        }
        for _ in image.height..padded_height {
            for _ in 0..padded_width {
                dots.push(black_pixel.clone());
            }
        }

        PaddedImage {
            width: image.width,
            height: image.height,
            padded_width,
            padded_height,
            dots,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        color::RGBColorFormat,
        image::{reader::padder::PaddedImage, Image},
    };

    #[test]
    fn pad_one() {
        let image: Image<f32> = Image {
            width: 1,
            height: 1,
            dots: Vec::from([RGBColorFormat::red()]),
        };
        let padded: PaddedImage = PaddedImage::new(image, 16, 8);
        assert_eq!(padded.dots.len(), 16 * 8);
        assert_eq!(padded.padded_height, 8);
        assert_eq!(padded.padded_width, 16);
        assert_eq!(padded.height, 1);
        assert_eq!(padded.width, 1)
    }

    #[test]
    fn pad_7_17() {
        let image: Image<f32> = Image {
            width: 17,
            height: 7,
            dots: Vec::from([RGBColorFormat::red(); 119]),
        };
        let padded: PaddedImage = PaddedImage::new(image, 16, 16);
        assert_eq!(padded.dots.len(), 32 * 16)
    }

    #[test]
    fn pad_99_99() {
        let image: Image<f32> = Image {
            width: 99,
            height: 99,
            dots: Vec::from([RGBColorFormat::red(); 9801]),
        };
        let padded: PaddedImage = PaddedImage::new(image, 10, 10);
        assert_eq!(padded.dots.len(), 10000)
    }
}
