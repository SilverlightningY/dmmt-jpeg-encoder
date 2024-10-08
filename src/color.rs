use core::panic;
use std::fmt::Display;

pub struct RGBColorFormat<T> {
    red: T,
    green: T,
    blue: T,
}

impl From<&RangeColorFormat<u16>> for RGBColorFormat<u8> {
    fn from(value: &RangeColorFormat<u16>) -> Self {
        RGBColorFormat {
            red: (value.red as f32 / value.max as f32 * u8::MAX as f32).floor() as u8,
            green: (value.green as f32 / value.max as f32 * u8::MAX as f32).floor() as u8,
            blue: (value.blue as f32 / value.max as f32 * u8::MAX as f32).floor() as u8,
        }
    }
}

pub struct RangeColorFormat<T> {
    max: T,
    red: T,
    green: T,
    blue: T,
}

impl<T: PartialOrd<T> + Display> RangeColorFormat<T> {
    pub fn new(max: T, red: T, green: T, blue: T) -> Self {
        if red > max || green > max || blue > max {
            panic!("Color value must not be greater than max value of {}", max);
        }
        RangeColorFormat {
            max,
            red,
            green,
            blue,
        }
    }
}

pub struct YCbCrColorFormat<T> {
    luma: T,
    chroma_blue: T,
    chroma_red: T,
}

impl From<&RGBColorFormat<u8>> for YCbCrColorFormat<u8> {
    fn from(value: &RGBColorFormat<u8>) -> Self {
        let red = value.red as f32 * 0.299_f32;
        let green = value.green as f32 * 0.587_f32;
        let blue = value.blue as f32 * 0.114_f32;
        let luma = red + green + blue;
        let red = value.red as f32 * -0.1687_f32;
        let green = value.green as f32 * 0.3312_f32;
        let blue = value.blue as f32 * 0.5_f32;
        let chroma_blue = red + green + blue + 0.5_f32;
        let red = value.red as f32 * 0.5_f32;
        let green = value.green as f32 * -0.4186_f32;
        let blue = value.blue as f32 * -0.0813_f32;
        let chroma_red = red + green + blue + 0.5_f32;

        YCbCrColorFormat {
            luma: luma.floor() as u8,
            chroma_blue: chroma_blue.floor() as u8,
            chroma_red: chroma_red.floor() as u8,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{RGBColorFormat, RangeColorFormat, YCbCrColorFormat};

    #[test]
    fn convert_rgb_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: 250_u8,
            green: 128_u8,
            blue: 14_u8,
        };
        let result: YCbCrColorFormat<u8> = YCbCrColorFormat::from(&rgb);
        assert_eq!(result.luma, 151, "luma is wrong");
        assert_eq!(result.chroma_blue, 7, "chroma blue is wrong");
        assert_eq!(result.chroma_red, 70, "chroma red is wrong");
    }

    #[test]
    fn convert_rgb_white_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: u8::MAX,
            green: u8::MAX,
            blue: u8::MAX,
        };
        let result = YCbCrColorFormat::from(&rgb);
        assert_eq!(result.luma, u8::MAX, "luma is wrong");
        assert_eq!(result.chroma_blue, 169_u8, "chroma blue is wrong");
        assert_eq!(result.chroma_red, 0_u8, "chroma red is wrong");
    }

    #[test]
    fn convert_rgb_black_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: 0_u8,
            green: 0_u8,
            blue: 0_u8,
        };
        let result = YCbCrColorFormat::from(&rgb);
        assert_eq!(result.luma, 0_u8, "luma is wrong");
        assert_eq!(result.chroma_blue, 0_u8, "chroma blue is wrong");
        assert_eq!(result.chroma_red, 0_u8, "chroma red is wrong");
    }

    #[test]
    fn convert_range_color_to_rgb() {
        let range_color = RangeColorFormat::new(17734_u16, 128_u16, 14355_u16, 9_u16);
        let result = RGBColorFormat::from(&range_color);
        assert_eq!(result.red, 1, "red is wrong");
        assert_eq!(result.green, 206, "green is wrong");
        assert_eq!(result.blue, 0, "blue is wrong");
    }

    #[test]
    fn convert_range_color_white_to_rgb() {
        let range_color = RangeColorFormat::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX);
        let result = RGBColorFormat::from(&range_color);
        assert_eq!(result.red, 255, "red is wrong");
        assert_eq!(result.green, 255, "green is wrong");
        assert_eq!(result.blue, 255, "blue is wrong");
    }

    #[test]
    fn convert_range_color_4bit_to_rgb() {
        let range_color = RangeColorFormat::new(15_u16, 2_u16, 5_u16, 15_u16);
        let result = RGBColorFormat::from(&range_color);
        assert_eq!(result.red, 34, "red is wrong");
        assert_eq!(result.green, 85, "green is wrong");
        assert_eq!(result.blue, 255, "blue is wrong");
    }

    #[test]
    #[should_panic]
    fn create_range_color_out_of_range() {
        RangeColorFormat::new(144, 12, 144, 145);
    }

    #[test]
    fn create_range_color() {
        RangeColorFormat::new(u16::MAX, 0, 5325, u16::MAX);
    }
}
