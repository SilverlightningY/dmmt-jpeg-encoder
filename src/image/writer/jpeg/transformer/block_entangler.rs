/**
 * Iterator to Fold two "lines" of an iterator in a 4x4 quad structure;
 * Example Use: Reorder the blocks of a subsampled JPEG image for output
 */
pub struct QuadFoldingIterator<'a, U: 'a, T: Iterator<Item = &'a U>> {
    linear_backlog: T,
    two_line_buffer: Vec<U>,
    two_line_buffer_index: usize,
    line_length: usize,
}

impl<'a, U: Copy + 'a, T: Iterator<Item = &'a U>> QuadFoldingIterator<'a, U, T> {
    pub fn new(linear_backlog: T, line_length: usize) -> Self {
        Self {
            linear_backlog,
            two_line_buffer: Vec::with_capacity(line_length * 2),
            two_line_buffer_index: line_length * 2,
            line_length,
        }
    }

    fn is_buffer_consumed(&self) -> bool {
        self.two_line_buffer_index >= self.two_line_buffer.len()
    }

    fn get_buffer_length(&self) -> usize {
        self.line_length * 2
    }

    fn refill_buffer(&mut self) {
        self.two_line_buffer_index = 0;
        self.two_line_buffer.clear();
        let mut items_pushed = 0;
        // Ans: For loops move the iterator (implicit call to into_iter()), which is NOT what
        //      we want here, as only part of the iterator is consumed by early break
        while let Some(&item) = self.linear_backlog.next() {
            self.two_line_buffer.push(item);
            items_pushed += 1;
            if items_pushed == self.get_buffer_length() {
                return;
            }
        }
        if items_pushed != 0 {
            panic!("Incomplete line at bottom of image, check padding!");
        }
    }

    fn get_next_block(&mut self) -> U {
        /*it's a kind of magic MAGIC magic ....*/
        let on_quad = self.two_line_buffer_index / 4;
        let line = (self.two_line_buffer_index % 4) / 2;
        let actual_index =
            self.two_line_buffer_index - (on_quad + line) * 2 + self.line_length * line;
        self.two_line_buffer_index += 1;
        self.two_line_buffer[actual_index]
    }
}

impl<'a, U: Default + Copy + 'a, T: Iterator<Item = &'a U>> Iterator
    for QuadFoldingIterator<'a, U, T>
{
    type Item = U;

    fn next(&mut self) -> Option<U> {
        if self.is_buffer_consumed() {
            self.refill_buffer();
        }
        if self.two_line_buffer.is_empty() {
            return None;
        }
        Some(self.get_next_block())
    }
}

#[cfg(test)]
mod tests {
    use super::QuadFoldingIterator;

    #[test]
    fn entangle_test() {
        let test_sequence: Vec<u32> = vec![0, 1, 4, 5, 2, 3, 6, 7, 8, 9, 12, 13, 10, 11, 14, 15];
        let expect_sequence: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let got_sequence: Vec<u32> = QuadFoldingIterator::new(test_sequence.iter(), 4).collect();
        for (&expect, &got) in expect_sequence.iter().zip(got_sequence.iter()) {
            assert_eq!(expect, got);
        }
    }
    #[test]
    fn entangle_test_assymetric() {
        let test_sequence: Vec<u32> = vec![
            0, 1, 4, 5, 8, 9, 2, 3, 6, 7, 10, 11, 12, 13, 16, 17, 20, 21, 14, 15, 18, 19, 22, 23,
        ];
        let expect_sequence: Vec<u32> = vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        ];
        let got_sequence: Vec<u32> = QuadFoldingIterator::new(test_sequence.iter(), 6).collect();
        for (&expect, &got) in expect_sequence.iter().zip(got_sequence.iter()) {
            assert_eq!(expect, got);
        }
    }
    #[test]
    #[should_panic]
    fn panic_test() {
        let test_sequence: Vec<u32> = vec![0, 1, 4, 5, 2, 3, 6, 7, 8, 9, 12, 13];
        let got_sequence: Vec<u32> = QuadFoldingIterator::new(test_sequence.iter(), 4).collect();
    }
}
