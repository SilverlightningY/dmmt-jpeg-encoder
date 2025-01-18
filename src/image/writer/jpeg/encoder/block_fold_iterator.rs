use crate::image::{
    subsampling::ChromaSubsamplingPreset,
    writer::jpeg::transformer::{categorize::CategorizedBlock, CombinedColorChannels},
};

pub enum ColorInformation {
    Luma,
    Chroma,
}

pub struct BlockFoldIterator<'a> {
    luma_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    chroma_blue_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    chroma_red_iterator: Box<dyn Iterator<Item = &'a CategorizedBlock> + 'a>,
    channel_selector: Box<dyn Iterator<Item = ColorChannelType>>,
}

impl<'a> BlockFoldIterator<'a> {
    pub fn new(
        channels: &'a CombinedColorChannels<Vec<CategorizedBlock>>,
        subsampling_preset: ChromaSubsamplingPreset,
    ) -> Self {
        let channel_selector: Box<dyn Iterator<Item = ColorChannelType>> = match subsampling_preset
        {
            ChromaSubsamplingPreset::P444 => Box::new(P444ChannelSelector::new()),
            ChromaSubsamplingPreset::P422 => Box::new(P422ChannelSelector::new()),
            ChromaSubsamplingPreset::P420 => Box::new(P420ChannelSelector::new()),
        };
        Self {
            luma_iterator: Box::new(channels.luma.iter()),
            chroma_blue_iterator: Box::new(channels.chroma_blue.iter()),
            chroma_red_iterator: Box::new(channels.chroma_red.iter()),
            channel_selector,
        }
    }

    fn take_next_luma_block(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let block = self.luma_iterator.next()?;
        Some((ColorInformation::Luma, block))
    }

    fn take_next_chroma_blue_block(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let block = self.chroma_blue_iterator.next()?;
        Some((ColorInformation::Chroma, block))
    }

    fn take_next_chroma_red_block(&mut self) -> Option<(ColorInformation, &'a CategorizedBlock)> {
        let block = self.chroma_red_iterator.next()?;
        Some((ColorInformation::Chroma, block))
    }
}

impl<'a> Iterator for BlockFoldIterator<'a> {
    type Item = (ColorInformation, &'a CategorizedBlock);

    fn next(&mut self) -> Option<Self::Item> {
        let next_channel = self
            .channel_selector
            .next()
            .expect("Channel selector must not end");
        match next_channel {
            ColorChannelType::Luma => self.take_next_luma_block(),
            ColorChannelType::ChromaBlue => self.take_next_chroma_blue_block(),
            ColorChannelType::ChromaRed => self.take_next_chroma_red_block(),
        }
    }
}

enum ColorChannelType {
    Luma,
    ChromaBlue,
    ChromaRed,
}

struct P444ChannelSelector {
    index: usize,
}

impl P444ChannelSelector {
    fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for P444ChannelSelector {
    type Item = ColorChannelType;

    fn next(&mut self) -> Option<Self::Item> {
        let return_value = match self.index {
            0 => ColorChannelType::Luma,
            1 => ColorChannelType::ChromaBlue,
            2 => ColorChannelType::ChromaRed,
            _ => panic!("Index to high"),
        };
        self.index = (self.index + 1) % 3;
        Some(return_value)
    }
}

struct P422ChannelSelector {
    index: usize,
}

impl P422ChannelSelector {
    fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for P422ChannelSelector {
    type Item = ColorChannelType;

    fn next(&mut self) -> Option<Self::Item> {
        let return_value = match self.index {
            0..=1 => ColorChannelType::Luma,
            2 => ColorChannelType::ChromaBlue,
            3 => ColorChannelType::ChromaRed,
            _ => panic!("Index to high"),
        };
        self.index = (self.index + 1) % 4;
        Some(return_value)
    }
}

struct P420ChannelSelector {
    index: usize,
}

impl P420ChannelSelector {
    fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for P420ChannelSelector {
    type Item = ColorChannelType;

    fn next(&mut self) -> Option<Self::Item> {
        let return_value = match self.index {
            0..=3 => ColorChannelType::Luma,
            4 => ColorChannelType::ChromaBlue,
            5 => ColorChannelType::ChromaRed,
            _ => panic!("Index too high"),
        };
        self.index = (self.index + 1) % 6;
        Some(return_value)
    }
}
