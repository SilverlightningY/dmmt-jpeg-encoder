use std::io::Read;
use std::str;

use super::super::Image;
use super::super::ImageReader;
use crate::color::{RGBColorFormat, RangeColorFormat};
use crate::Error;

pub struct PPMImageReader<T: Read> {
    reader: T,
}

impl<T: Read> PPMImageReader<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }
}

impl<T: Read> ImageReader<f32> for PPMImageReader<T> {
    fn read_image(&mut self) -> crate::Result<Image<f32>> {
        let mut tokenizer = PPMTokenizer::new(&mut self.reader);
        let mut parser = PPMParser::new(&mut tokenizer);
        parser.parse_tokens()
    }
}

struct PPMTokenizer<'a, R: Read> {
    reader: &'a mut R,
    buffer: Vec<u8>,
}

impl<'a, R: Read> PPMTokenizer<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        PPMTokenizer {
            reader,
            buffer: Vec::new(),
        }
    }
}

impl<R: Read> Iterator for PPMTokenizer<'_, R> {
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

const P3_HEADER_TOKEN_NAME: &str = "P3 Header";
const WIDTH_HEADER_TOKEN_NAME: &str = "Width Header";
const HEIGHT_HEADER_TOKEN_NAME: &str = "Height Header";
const MAX_VALUE_HEADER_TOKEN_NAME: &str = "Max Value Header";
const COLOR_COMPONENT_VALUE_TOKEN_NAME: &str = "Color Component Value";

#[derive(Clone, Copy)]
struct Dot {
    buffer: [u16; 3],
    index: usize,
}

impl Dot {
    fn new() -> Self {
        Self {
            buffer: [u16::default(); 3],
            index: 0,
        }
    }

    fn red(&self) -> u16 {
        self.buffer[0]
    }

    fn green(&self) -> u16 {
        self.buffer[1]
    }

    fn blue(&self) -> u16 {
        self.buffer[2]
    }

    fn push_color_component(&mut self, component: u16) {
        if self.is_complete() {
            return;
        }
        self.buffer[self.index] = component;
        self.index += 1;
    }

    fn is_complete(&self) -> bool {
        self.index == 3
    }

    fn reset(&mut self) {
        self.index = 0;
    }

    fn is_empty(&self) -> bool {
        self.index == 0
    }
}

struct PPMParser<'a, T> {
    tokenizer: &'a mut T,
}

impl<'a, T> PPMParser<'a, T>
where
    T: Iterator<Item = String>,
{
    fn new(tokenizer: &'a mut T) -> Self {
        Self { tokenizer }
    }

    fn parse_tokens(&mut self) -> crate::Result<Image<f32>> {
        let header = self.parse_header()?;
        Self::check_header_version(&header)?;
        let width = self.parse_width()?;
        let height = self.parse_height()?;
        let max_value = self.parse_max_value()?;
        let dots = self.parse_all_dots()?;
        Self::check_parsed_dots_length_match_header_information(&dots, width, height)?;
        let dots = dots
            .into_iter()
            .map(|d| RangeColorFormat::new(max_value, d.red(), d.green(), d.blue()))
            .map(RGBColorFormat::from)
            .collect::<Vec<RGBColorFormat<f32>>>();
        Ok(Image {
            width,
            height,
            dots,
        })
    }

    fn check_parsed_dots_length_match_header_information(
        dots: &[Dot],
        width: u16,
        height: u16,
    ) -> crate::Result<()> {
        let expected_number_of_dots = width as usize * height as usize;
        if dots.len() != expected_number_of_dots {
            return Err(Error::MismatchOfSizeBetweenHeaderAndValues);
        }
        Ok(())
    }

    fn check_header_version(header: &str) -> crate::Result<()> {
        if header != "P3" {
            return Err(Error::PPMFileDoesNotContainRequiredToken(
                P3_HEADER_TOKEN_NAME,
            ));
        }
        Ok(())
    }

    fn parse_header(&mut self) -> crate::Result<String> {
        self.tokenizer
            .next()
            .ok_or(Error::PPMFileDoesNotContainRequiredToken(
                P3_HEADER_TOKEN_NAME,
            ))
    }

    fn parse_width(&mut self) -> crate::Result<u16> {
        self.tokenizer
            .next()
            .ok_or(Error::PPMFileDoesNotContainRequiredToken(
                WIDTH_HEADER_TOKEN_NAME,
            ))?
            .parse()
            .map_err(|_| Error::ParsingOfTokenFailed(WIDTH_HEADER_TOKEN_NAME))
    }

    fn parse_height(&mut self) -> crate::Result<u16> {
        self.tokenizer
            .next()
            .ok_or(Error::PPMFileDoesNotContainRequiredToken(
                HEIGHT_HEADER_TOKEN_NAME,
            ))?
            .parse()
            .map_err(|_| Error::ParsingOfTokenFailed(HEIGHT_HEADER_TOKEN_NAME))
    }

    fn parse_max_value(&mut self) -> crate::Result<u16> {
        self.tokenizer
            .next()
            .ok_or(Error::PPMFileDoesNotContainRequiredToken(
                MAX_VALUE_HEADER_TOKEN_NAME,
            ))?
            .parse()
            .map_err(|_| Error::ParsingOfTokenFailed(MAX_VALUE_HEADER_TOKEN_NAME))
    }

    fn parse_all_dots(&mut self) -> crate::Result<Vec<Dot>> {
        let mut current_dot = Dot::new();
        let mut dots = Vec::new();
        for token in self.tokenizer.by_ref() {
            let component = Self::parse_color_value(&token)?;
            current_dot.push_color_component(component);
            if current_dot.is_complete() {
                dots.push(current_dot);
                current_dot.reset();
            }
        }
        Self::check_pixel_was_complete(&current_dot)?;
        Ok(dots)
    }

    fn check_pixel_was_complete(dot: &Dot) -> crate::Result<()> {
        if !dot.is_empty() {
            return Err(Error::IncompletePixelParsed(dot.index));
        }
        Ok(())
    }

    fn parse_color_value(token: &str) -> crate::Result<u16> {
        token
            .parse()
            .map_err(|_| Error::ParsingOfTokenFailed(COLOR_COMPONENT_VALUE_TOKEN_NAME))
    }
}

#[cfg(test)]
mod test {
    use crate::{error::Error, image::Image, Result};

    use super::{PPMParser, PPMTokenizer};

    fn parse_ppm_tokens(token_string: &str) -> Result<Image<f32>> {
        let mut bytes = token_string.as_bytes();
        let mut tokenizer = PPMTokenizer::new(&mut bytes);
        let mut parser = PPMParser::new(&mut tokenizer);
        parser.parse_tokens()
    }

    #[test]
    fn read_string() {
        let string = "P3\n# Example PPM image string\n3 2\n255\n255 0 0   0 255 0   0 0 255\n255 255 0  255 0 255  0 255 255";
        let image = parse_ppm_tokens(&string).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    fn read_continuous_string() {
        let string = "P3 3 2 255 255 0 0   0 255 0   0 0 255 255 255 0  255 0 255  0 255 255";
        let image = parse_ppm_tokens(&string).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    fn read_newline_string() {
        let string = "P3\n# Example PPM image newlines\n3\n2\n255\n255\n0\n0\n0\n255\n0\n0\n0\n255\n255\n255\n0\n255\n0\n255\n0\n255\n255";
        let image = parse_ppm_tokens(&string).unwrap();
        assert!(image.height == 2);
    }

    #[test]
    fn incomplete_pixel() {
        let string = "P3\n3 2 255 0 0 255 0 0";
        if let Err(Error::IncompletePixelParsed(n)) = parse_ppm_tokens(&string) {
            if n != 2 {
                panic!("Number of parsed pixels should be 2, but was {}", n);
            }
            return;
        };
        panic!("Incomplete pixel not detected");
    }

    #[test]
    fn wrong_size() {
        let string = "P3\n3 2 255 0 0 255";
        if let Err(Error::MismatchOfSizeBetweenHeaderAndValues) = parse_ppm_tokens(&string) {
            return;
        };
        panic!("Mismatch of size in header and actual pixels was not detected!");
    }
}
