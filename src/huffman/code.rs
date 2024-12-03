use super::{SymbolCodeLength, SymbolFrequency};

pub type HuffmanCode = Vec<usize>;

pub trait HuffmanCodeGenerator {
    fn generate(&mut self, sorted_frequencies: &[usize]) -> HuffmanCode;

    fn generate_with_symbols(
        &mut self,
        sorted_frequencies: &[SymbolFrequency],
    ) -> Vec<SymbolCodeLength> {
        let frequencies: Vec<usize> = sorted_frequencies.iter().map(|f| f.frequency).collect();
        let code = self.generate(&frequencies);
        code.into_iter()
            .zip(sorted_frequencies)
            .map(|(length, sf)| SymbolCodeLength {
                symbol: sf.symbol,
                length,
            })
            .collect()
    }
}
