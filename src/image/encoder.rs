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
        self.write_exif_application_header()?;
        self.write_luminance_quantization_table()?;
        self.write_chrominance_quantization_table()?;
        self.write_start_of_frame()?;
        // write huffman tables
        self.write_start_of_scan()?;
        self.write_image_data()?;
        self.write_end_of_file()?;
        Ok(())
    }

    fn write_segment(&mut self, marker: SegmentMarker, content: &[u8]) -> io::Result<()> {
        let marker = marker.as_binary_ref();
        let segment_length = (marker.len() + content.len()).to_be_bytes();
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

    fn write_exif_application_header(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::ExifApplication, &[])
    }

    fn write_luminance_quantization_table(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
    }

    fn write_chrominance_quantization_table(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
    }

    fn write_start_of_frame(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::StartOfFrame, &[])
    }

    fn write_start_of_scan(&mut self) -> io::Result<()> {
        self.write_segment(SegmentMarker::StartOfScan, &[])
    }

    fn write_image_data(&mut self) -> io::Result<()> {
        todo!("implement write image data");
    }
}
