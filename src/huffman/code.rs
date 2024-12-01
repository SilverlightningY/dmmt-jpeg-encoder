pub type HuffmanCode = Vec<usize>;

pub trait HuffmanCodeGenerator {
    fn generate(&mut self, sorted_frequencies: &[usize]) -> HuffmanCode;
}
