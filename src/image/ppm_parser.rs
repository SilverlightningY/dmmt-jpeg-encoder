use super::Image;
use crate::color::{RGBColorFormat, RangeColorFormat, YCbCrColorFormat};
use std::io::Read;
use std::str;

pub struct PPMTokenizer<R: Read> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: Read> PPMTokenizer<R> {
    pub fn new(reader: R) -> Self {
        PPMTokenizer {
            reader,
            buffer: Vec::new(),
        }
    }
}

impl<R: Read> Iterator for PPMTokenizer<R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();
        let mut byte = [0; 1];
        let mut in_comment = false;

        while self.reader.read(&mut byte).unwrap_or(0) > 0 {
            if in_comment {
                if byte[0] == b'\n' {
                    in_comment = false;
                }
                continue;
            }
            if byte[0] == b'#' {
                in_comment = true;
                continue;
            }
            if byte[0].is_ascii_whitespace() {
                if !self.buffer.is_empty() {
                    break;
                }
            } else {
                self.buffer.push(byte[0]);
            }
        }

        if self.buffer.is_empty() {
            return None;
        }

        let token = str::from_utf8(&self.buffer)
            .expect("Invalid UTF-8 sequence")
            .to_string();
        Some(token)
    }
}

struct PPMParser {}

impl PPMParser {
    pub fn parse<R: Iterator<Item = String>>(mut tokenizer: R) -> Result<Image<f32>, String> {
        let mut luma: Vec<f32> = Vec::new();
        let mut chroma_blue: Vec<f32> = Vec::new();
        let mut chroma_red: Vec<f32> = Vec::new();

        let header = tokenizer.next().unwrap();
        if header != "P3" {
            return Err("Image File does not start with 'P3'".to_string());
        }
        let width = tokenizer.next().unwrap().parse().unwrap_or(0);
        let height = tokenizer.next().unwrap().parse().unwrap_or(0);
        let max = tokenizer.next().unwrap().parse().unwrap_or(0);

        let mut pixel = Vec::with_capacity(3);

        for token in tokenizer {
            pixel.push(token.parse().unwrap());
            if pixel.len() == 3 {
                let col: RangeColorFormat<u16> =
                    RangeColorFormat::new(max, pixel[0], pixel[1], pixel[2]);
                let rgb = RGBColorFormat::from(&col);
                let result: YCbCrColorFormat<f32> = YCbCrColorFormat::from(&rgb);
                luma.push(result.luma);
                chroma_blue.push(result.chroma_blue);
                chroma_red.push(result.chroma_red);
                pixel.clear();
            }
        }

        if pixel.len() != 0 {
            return Err("Invalid number of rgb values. Incomplete pixel".to_string());
        }
        if width as u32 * height as u32 != luma.len() as u32 {
            return Err("Size of image in header does not match amount of pixels".to_string());
        }

        Ok(Image::<f32> {
            width,
            height,
            luma,
            chroma_blue,
            chroma_red,
        })
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::BufReader};

    use super::{PPMParser, PPMTokenizer};

    #[test]
    fn read_image() {
        let img_path = "src/image.ppm";
        let file = File::open(img_path).expect("Failed to open file");
        let reader = BufReader::new(file);
        let image = PPMParser::parse(PPMTokenizer::new(reader)).unwrap();
        assert!(image.height == 480);
    }

    #[test]
    fn read_string() {
        let string = "P3\n# Example PPM image string\n3 2\n255\n255 0 0   0 255 0   0 0 255\n255 255 0  255 0 255  0 255 255";
        let image = PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    fn read_continuous_string() {
        let string = "P3 3 2 255 255 0 0   0 255 0   0 0 255 255 255 0  255 0 255  0 255 255";
        let image = PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    fn read_newline_string() {
        let string = "P3\n# Example PPM image newlines\n3\n2\n255\n255\n0\n0\n0\n255\n0\n0\n0\n255\n255\n255\n0\n255\n0\n255\n0\n255\n255";
        let image = PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    #[should_panic(expected = "Invalid number of rgb values. Incomplete pixel")]
    fn incomplete_pixel() {
        let string = "P3\n3 2 255 0 0 255 0 0";
        PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
    }

    #[test]
    #[should_panic(expected = "Size of image in header does not match amount of pixels")]
    fn wrong_size() {
        let string = "P3\n3 2 255 0 0 255";
        PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
    }
}
