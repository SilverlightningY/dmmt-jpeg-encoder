use std::io::Write;

pub struct SegmentMarkerInjector<'a, T: Write> {
    writer: &'a mut T,
}

impl<'a, T: Write> SegmentMarkerInjector<'a, T> {
    pub fn new(writer: &'a mut T) -> Self {
        Self { writer }
    }
}

impl<T: Write> Write for SegmentMarkerInjector<'_, T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written = 0;
        for &b in buf {
            let n = self.writer.write(&[b])?;
            if n == 0 {
                return Ok(bytes_written);
            }
            bytes_written += 1;
            if b == 0xFF {
                let n = self.writer.write(&[0])?;
                if n == 0 {
                    panic!("Could not inject 0x00 into stream after 0xFF, as underlying writer does not accept further bytes");
                }
            }
        }
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::SegmentMarkerInjector;

    #[test]
    fn injector_test() {
        let test_sequence: Vec<u8> = vec![0x01, 0x02, 0xFF, 0x00, 0x03];
        let expect_sequence: Vec<u8> = vec![0x01, 0x02, 0xFF, 0x00, 0x00, 0x03];

        let mut output_sequence: Vec<u8> = Vec::new();

        let mut writer = SegmentMarkerInjector::new(&mut output_sequence);
        writer.write_all(&test_sequence).expect("writing failed");

        assert_eq!(expect_sequence.len(), output_sequence.len());

        for (&expect, &got) in expect_sequence.iter().zip(output_sequence.iter()) {
            assert_eq!(expect, got);
        }
    }
}
