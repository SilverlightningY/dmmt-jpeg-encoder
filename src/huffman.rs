pub mod code;
pub mod coding_error;
pub mod encoder;
pub mod length_limited;
pub mod tree;

pub type Symbol = u8;

#[derive(Debug)]
pub struct SymbolCodeLength {
    pub symbol: Symbol,
    pub length: usize,
}

impl SymbolCodeLength {
    pub fn new(symbol: Symbol, length: usize) -> Self {
        Self { symbol, length }
    }

    pub fn len(&self) -> usize {
        self.length as usize
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl From<(Symbol, usize)> for SymbolCodeLength {
    fn from(value: (Symbol, usize)) -> Self {
        Self {
            symbol: value.0,
            length: value.1,
        }
    }
}

#[derive(Clone)]
pub struct SymbolFrequency {
    pub symbol: Symbol,
    pub frequency: usize,
}

impl SymbolFrequency {
    pub fn new(symbol: Symbol, frequency: usize) -> Self {
        Self { symbol, frequency }
    }
}

impl From<(Symbol, usize)> for SymbolFrequency {
    fn from(value: (Symbol, usize)) -> Self {
        Self {
            symbol: value.0,
            frequency: value.1,
        }
    }
}
