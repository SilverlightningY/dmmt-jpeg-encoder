use crate::image::{
    subsampling::ChromaSubsamplingPreset,
    writer::jpeg::transformer::{categorize::CategorizedBlock, CombinedColorChannels},
};

enum Movement {
    Up,
    Right,
    Down,
    Left,
}

pub enum ColorInformation {
    Luma,
    Chroma,
}

pub struct BlockFoldIterator<'a> {
    luma_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    chroma_blue_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    chroma_red_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    subsampling_preset: ChromaSubsamplingPreset,
    index: usize,
}

impl<'a> BlockFoldIterator<'a> {
    pub fn new(
        channels: &'a CombinedColorChannels<Vec<CategorizedBlock>>,
        subsampling_preset: ChromaSubsamplingPreset,
    ) -> Self {
        let index = 0;
        match subsampling_preset {
            ChromaSubsamplingPreset::P444 => Self {
                luma_iterator: Box::new(channels.luma.iter()),
                chroma_blue_iterator: Box::new(channels.chroma_blue.iter()),
                chroma_red_iterator: Box::new(channels.chroma_red.iter()),
                subsampling_preset,
                index,
            },
            _ => todo!("Not implemented"),
        }
    }

    fn next_block_for_p444(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let return_value = match self.index {
            0 => self
                .luma_iterator
                .next()
                .map(|i| (ColorInformation::Luma, i)),
            1 => self
                .chroma_blue_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            2 => self
                .chroma_red_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            _ => panic!("Index to high"),
        };
        self.index = (self.index + 1) % 3;
        return_value
    }

    fn next_block_for_p422(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let return_value = match self.index {
            0..=1 => self
                .luma_iterator
                .next()
                .map(|i| (ColorInformation::Luma, i)),
            2 => self
                .chroma_blue_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            3 => self
                .chroma_red_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            _ => panic!("Index to high"),
        };
        self.index = (self.index + 1) % 4;
        return_value
    }

    fn next_block_for_p420(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let return_value = match self.index {
            0..=3 => self
                .luma_iterator
                .next()
                .map(|i| (ColorInformation::Luma, i)),
            4 => self
                .chroma_blue_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            5 => self
                .chroma_red_iterator
                .next()
                .map(|i| (ColorInformation::Chroma, i)),
            _ => panic!("Index to high"),
        };
        self.index = (self.index + 1) % 6;
        return_value
    }
}

impl<'a> Iterator for BlockFoldIterator<'a> {
    type Item = (ColorInformation, &'a CategorizedBlock);

    fn next(&mut self) -> Option<Self::Item> {
        match self.subsampling_preset {
            ChromaSubsamplingPreset::P444 => self.next_block_for_p444(),
            ChromaSubsamplingPreset::P422 => self.next_block_for_p422(),
            ChromaSubsamplingPreset::P420 => self.next_block_for_p420(),
        }
    }
}

trait IterationSchema: Iterator<Item = Movement> {
    fn row_stepwidth() -> usize;
}

struct ChannelIterator<'a, I> {
    channel: &'a [CategorizedBlock],
    iteration_schema: I,
}

// impl<'a, I> ChannelIterator<'a>
// where
//     I: Iterator<Item = Movement>,
// {
//     fn new() {}
// }
