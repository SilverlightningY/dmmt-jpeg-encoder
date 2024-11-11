use crate::error::Error;
use crate::Result;
use core::panic;
use std::fmt::Display;
use std::io;
use std::io::Write;

use super::OutputImage;
use crate::logger;

pub struct Encoder<'a, T> {
    image: &'a OutputImage,
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

#[derive(Debug)]
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

impl Display for SegmentMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HuffmanTable => write!(f, "Huffman Table"),
            Self::QuantizationTable => write!(f, "Quantization Table"),
            Self::ExifApplication => write!(f, "Exif Application"),
            Self::JfifApplication => write!(f, "Jfif Application"),
            Self::StartOfFrame => write!(f, "Start of Frame"),
            Self::StartOfScan => write!(f, "Start of Scan"),
        }
    }
}

impl<'a, T: Write> Encoder<'a, T> {
    pub fn new(image: &'a OutputImage, writer: &'a mut T) -> Encoder<'a, T> {
        Encoder { image, writer }
    }

    pub fn encode(&mut self) -> Result<()> {
        self.write_start_of_file()?;
        self.write_jfif_application_header()?;
        // self.write_luminance_quantization_table()?;
        // self.write_chrominance_quantization_table()?;
        // self.write_start_of_frame()?;
        // write huffman tables
        // self.write_start_of_scan()?;
        // self.write_image_data()?;
        self.write_end_of_file()?;
        Ok(())
    }

    fn write_segment(&mut self, marker: SegmentMarker, content: &[u8]) -> io::Result<()> {
        log::info!("Writing {:?}", marker);
        let marker_binary_ref = marker.as_binary_ref();
        let segment_len = marker_binary_ref.len() + content.len();
        if segment_len > u16::MAX as usize {
            panic!(
                "The length of the segment '{}' is greater than u16::MAX",
                marker
            );
        }
        let segment_length = (segment_len as u16).to_be_bytes();
        logger::log_segment(&marker_binary_ref, &content, &segment_length);
        self.writer.write_all(marker_binary_ref)?;
        self.writer.write_all(&segment_length)?;
        self.writer.write_all(content)?;
        Ok(())
    }

    fn write_control_marker(&mut self, marker: ControlMarker) -> io::Result<()> {
        self.writer.write_all(marker.as_binary_ref())
    }

    fn write_start_of_file(&mut self) -> Result<()> {
        self.write_control_marker(ControlMarker::StartOfFile)
            .map_err(|_| Error::FailedToWriteStartOfFile)
    }

    fn write_end_of_file(&mut self) -> Result<()> {
        self.write_control_marker(ControlMarker::EndOfFile)
            .map_err(|_| Error::FailedToWriteEndOfFile)
    }

    fn write_jfif_application_header(&mut self) -> Result<()> {
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
            .map_err(|_| Error::FailedToWriteJfifApplicationHeader)
    }

    fn write_luminance_quantization_table(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
            .map_err(|_| Error::FailedToWriteLuminanceQuantizationTable)
    }

    fn write_chrominance_quantization_table(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
            .map_err(|_| Error::FailedToWriteChrominanceQuantizationTable)
    }

    fn write_start_of_frame(&mut self) -> Result<()> {
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
            .map_err(|_| Error::FailedToWriteStartOfFrame)
    }

    fn write_start_of_scan(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::StartOfScan, &[])
            .map_err(|_| Error::FailedToWriteStartOfScan)
    }

    fn write_image_data(&mut self) -> Result<()> {
        todo!("implement write image data");
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;
    use crate::image::ChannelSubsamplingMethod::Skip;
    use crate::image::ChromaSubsamplingPreset::P444;
    use crate::image::{
        ppm_parser::{PPMParser, PPMTokenizer},
        transformer::JpegTransformer,
        TransformationOptions,
    };
    use std::fs::File;

    #[test]
    fn test_write_file() {
        let string = "P3 3 2 255 255 0 0   0 255 0   0 0 255 255 255 0  255 0 255  0 255 255";
        let image = PPMParser::parse(PPMTokenizer::new(string.as_bytes())).unwrap();
        let options = TransformationOptions {
            chroma_subsampling_preset: P444,
            bits_per_channel: 8,
            chroma_subsampling_method: Skip,
        };
        let transformed_image = JpegTransformer::new(&image).transform(&options).unwrap();

        let output_path = "out/output_image.jpg";
        let mut output_file = File::create(output_path).expect("Failed to create output file");
        let mut encoder: Encoder<std::fs::File> = Encoder {
            image: &transformed_image,
            writer: &mut output_file,
        };
        encoder.encode().expect("Failed to encode image");
    }
}
