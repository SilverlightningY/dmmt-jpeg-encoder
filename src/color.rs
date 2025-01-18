use core::panic;
use std::fmt::Display;

#[derive(Clone, Copy)]
pub struct RGBColorFormat<T> {
    red: T,
    green: T,
    blue: T,
}

pub struct RangeColorFormat<T> {
    max: T,
    red: T,
    green: T,
    blue: T,
}

pub struct YCbCrColorFormat<T> {
    pub luma: T,
    pub chroma_blue: T,
    pub chroma_red: T,
}

#[cfg(test)]
impl RGBColorFormat<f32> {
    pub fn red() -> Self {
        RGBColorFormat {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        }
    }
}

impl Default for RGBColorFormat<f32> {
    fn default() -> Self {
        RGBColorFormat {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
        }
    }
}

impl From<&RangeColorFormat<u16>> for RGBColorFormat<f32> {
    fn from(value: &RangeColorFormat<u16>) -> Self {
        RGBColorFormat {
            red: value.red as f32 / value.max as f32,
            green: value.green as f32 / value.max as f32,
            blue: value.blue as f32 / value.max as f32,
        }
    }
}

impl From<RangeColorFormat<u16>> for RGBColorFormat<f32> {
    fn from(value: RangeColorFormat<u16>) -> Self {
        RGBColorFormat::from(&value)
    }
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

impl From<&RGBColorFormat<f32>> for YCbCrColorFormat<f32> {
    fn from(value: &RGBColorFormat<f32>) -> Self {
        let red = value.red;
        let green = value.green;
        let blue = value.blue;

        let weighted_red = red * 0.299_f32;
        let weighted_green = green * 0.587_f32;
        let weighted_blue = blue * 0.114_f32;
        let luma = (weighted_red + weighted_green + weighted_blue - 128_f32 / 255_f32) * 255_f32;
        let weighted_red = red * -0.1687_f32;
        let weighted_green = green * -0.3312_f32;
        let weighted_blue = blue * 0.5_f32;
        let chroma_blue = (weighted_red + weighted_green + weighted_blue) * 255_f32;
        let weighted_red = red * 0.5_f32;
        let weighted_green = green * -0.4186_f32;
        let weighted_blue = blue * -0.0813_f32;
        let chroma_red = (weighted_red + weighted_green + weighted_blue) * 255_f32;

        YCbCrColorFormat {
            luma,
            chroma_blue,
            chroma_red,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{RGBColorFormat, RangeColorFormat, YCbCrColorFormat};

    #[test]
    fn convert_rgb_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: 0.25_f32,
            green: 0.75_f32,
            blue: 0.333_f32,
        };
        let result = YCbCrColorFormat::from(&rgb);
        assert!(
            result.luma >= 12.95_f32 && result.luma < 13.05_f32,
            "luma is wrong, was {}",
            result.luma
        );
        assert!(
            result.chroma_blue >= -31.68 && result.chroma_blue < -31.58,
            "chroma blue is wrong, was {}",
            result.chroma_blue
        );
        assert!(
            result.chroma_red >= -55.13 && result.chroma_red < -55.03,
            "chroma red is wrong, was {}",
            result.chroma_red
        );
    }

    #[test]
    fn convert_rgb_white_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: 1_f32,
            green: 1_f32,
            blue: 1_f32,
        };
        let result = YCbCrColorFormat::from(&rgb);
        assert!(
            result.luma >= 126.99999_f32 && result.luma <= 127.00001_f32,
            "luma is wrong, was {}",
            result.luma
        );
        assert!(
            result.chroma_blue >= -0.5_f32 && result.chroma_blue <= 0.5_f32,
            "chroma blue is wrong, was {}",
            result.chroma_blue
        );
        assert!(
            result.chroma_red >= -0.5_f32 && result.chroma_red <= 0.5_f32,
            "chroma red is wrong, was {}",
            result.chroma_red
        );
    }

    #[test]
    fn convert_rgb_black_to_ycbcr() {
        let rgb = RGBColorFormat {
            red: 0_f32,
            green: 0_f32,
            blue: 0_f32,
        };
        let result = YCbCrColorFormat::from(&rgb);
        assert_eq!(result.luma, -128_f32, "luma is wrong");
        assert_eq!(result.chroma_blue, 0_f32, "chroma blue is wrong");
        assert_eq!(result.chroma_red, 0_f32, "chroma red is wrong");
    }

    #[test]
    fn convert_range_color_to_rgb() {
        let range_color = RangeColorFormat::new(17734_u16, 128_u16, 14355_u16, 9_u16);
        let result = RGBColorFormat::from(&range_color);
        assert!(
            result.red >= 7.209e-3_f32 && result.red <= 7.219e-3_f32,
            "red is wrong"
        );
        assert!(
            result.green >= 0.809459_f32 && result.red <= 0.809469_f32,
            "green is wrong"
        );
        assert!(
            result.blue >= 4.99e-4_f32 && result.blue <= 5.09e-4_f32,
            "blue is wrong"
        );
    }

    #[test]
    fn convert_range_color_white_to_rgb() {
        let range_color = RangeColorFormat::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX);
        let result = RGBColorFormat::from(&range_color);
        assert_eq!(result.red, 1_f32, "red is wrong");
        assert_eq!(result.green, 1_f32, "green is wrong");
        assert_eq!(result.blue, 1_f32, "blue is wrong");
    }

    #[test]
    fn convert_range_color_4bit_to_rgb() {
        let range_color = RangeColorFormat::new(0b1111_u16, 0b0010_u16, 0b0101_u16, 0b1111_u16);
        let result = RGBColorFormat::from(&range_color);
        assert!(
            result.red >= 0.133333 && result.red <= 0.133334,
            "red is wrong"
        );
        assert!(
            result.green >= 0.333333 && result.green <= 0.333334,
            "green is wrong"
        );
        assert!(result.blue == 1_f32, "blue is wrong");
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
