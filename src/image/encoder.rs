use std::io;
use std::io::Write;

use super::Image;

pub struct Encoder<'a, T, I> {
    image: &'a Image<I>,
    writer: &'a mut T,
}

const START_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD8];
const END_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD9];
const HUFFMAN_TABLE_MARKER: [u8; 2] = [0xFF, 0xC4];
const QUANTIZATION_TABLE_MARKER: [u8; 2] = [0xFF, 0xDB];
const START_OF_FRAME_MARKER: [u8; 2] = [0xFF, 0xC0];
const START_OF_SCAN_MARKER: [u8; 2] = [0xFF, 0xDA];
const EXIF_APPLICATION_MARKER: [u8; 2] = [0xFF, 0xE1];
const JFIF_APPLICATION_MARKER: [u8; 2] = [0xFF, 0xE0];

enum ControlMarker {
    StartOfFile,
    EndOfFile,
}

enum SegmentMarker {
    HuffmanTable,
    QuantizationTable,
    ExifApplication,
    JfifApplication,
    StartOfFrame,
    StartOfScan,
}

trait AsBinaryRef {
    fn as_binary_ref(&self) -> &'static [u8];
}

impl AsBinaryRef for ControlMarker {
    fn as_binary_ref(&self) -> &'static [u8] {
        match self {
            Self::StartOfFile => &START_OF_FILE_MARKER,
            Self::EndOfFile => &END_OF_FILE_MARKER,
        }
    }
}

impl AsBinaryRef for SegmentMarker {
    fn as_binary_ref(&self) -> &'static [u8] {
        match self {
            Self::HuffmanTable => &HUFFMAN_TABLE_MARKER,
            Self::QuantizationTable => &QUANTIZATION_TABLE_MARKER,
            Self::ExifApplication => &EXIF_APPLICATION_MARKER,
            Self::JfifApplication => &JFIF_APPLICATION_MARKER,
            Self::StartOfFrame => &START_OF_FRAME_MARKER,
            Self::StartOfScan => &START_OF_SCAN_MARKER,
        }
    }
}

impl<'a, T: Write, I> Encoder<'a, T, I> {
    pub fn new(image: &'a Image<I>, writer: &'a mut T) -> Encoder<'a, T, I> {
        Encoder { image, writer }
    }

    pub fn encode(&mut self) -> io::Result<()> {
        self.write_start_of_file()?;
        self.write_jfif_application_header()?;
        self.write_luminance_quantization_table()?;
        self.write_chrominance_quantization_table()?;
        self.write_start_of_frame()?;
        // write huffman tables
        // self.write_start_of_scan()?;
        // self.write_image_data()?;
        self.write_end_of_file()?;
        Ok(())
    }

    fn write_segment(&mut self, marker: SegmentMarker, content: &[u8]) -> io::Result<()> {
        let marker = marker.as_binary_ref();
        let segment_length = (marker.len() as u16 + content.len() as u16).to_be_bytes();
        self.writer.write_all(marker)?;
        self.writer.write_all(&segment_length)?;
        self.writer.write_all(content)?;
        Ok(())
    }

    fn write_control_marker(&mut self, marker: ControlMarker) -> io::Result<()> {
        self.writer.write_all(marker.as_binary_ref())
    }

    fn write_start_of_file(&mut self) -> io::Result<()> {
        self.write_control_marker(ControlMarker::StartOfFile)
    }

    fn write_end_of_file(&mut self) -> io::Result<()> {
        self.write_control_marker(ControlMarker::EndOfFile)
    }

    fn write_jfif_application_header(&mut self) -> io::Result<()> {
        let width_bytes = self.image.width.to_be_bytes();
        let height_bytes = self.image.height.to_be_bytes();
        #[rustfmt::skip]
        let content = &[
            b'J', b'F', b'I', b'F', b'\0',// Identifier
            0x01, 0x02,             // Version
            0x00,                   // Density
            width_bytes[0], width_bytes[1], // X Density
            height_bytes[0], height_bytes[1], // Y Density
            0,                      // X Thumbnail
            0                       // Y Thumbnail
        ];
        self.write_segment(SegmentMarker::JfifApplication, content)
    }

    fn write_luminance_quantization_table(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
    }

    fn write_chrominance_quantization_table(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
    }

    fn write_start_of_frame(&mut self) -> io::Result<()> {
        let width_bytes = self.image.width.to_be_bytes();
        let height_bytes = self.image.height.to_be_bytes();
        #[rustfmt::skip]
        let content = &[
            0x08,                   // bits per pixel
            height_bytes[0], height_bytes[1], // image height
            width_bytes[0], width_bytes[1],   // image width
            0x03,                   // components (1 or 3)
            0x01, 0x22, 0x00,       // 0x01=y component, sampling factor, quant. table
            0x02, 0x11, 0x01,       // 0x02=Cb component, ...
            0x03, 0x11, 0x01,       // 0x03=Cr component, ...
        ];
        self.write_segment(SegmentMarker::StartOfFrame, content)
    }

    fn write_start_of_scan(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::StartOfScan, &[])
    }

    fn write_image_data(&mut self) -> io::Result<()> {
        todo!("implement write image data");
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use super::Encoder;
    use crate::image::ppm_parser::{PPMParser, PPMTokenizer};

    #[test]
    fn test_write_file() {
        let string = "P3 3 2 255 255 0 0   0 255 0   0 0 255 255 255 0  255 0 255  0 255 255";
        let image = PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();

        let output_path = "tests/output_image.jpg";
        let mut output_file = File::create(output_path).expect("Failed to create output file");
        let mut encoder: Encoder<std::fs::File, f32> = Encoder {
            image: &image,
            writer: &mut output_file,
        };
        encoder.encode().expect("Failed to encode image");
    }
}
