use crate::image::{
    subsampling::ChromaSubsamplingPreset,
    writer::jpeg::transformer::{categorize::CategorizedBlock, CombinedColorChannels},
};

#[derive(Clone, Copy)]
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
        image_width: usize,
    ) -> Self {
        let index = 0;
        match subsampling_preset {
            ChromaSubsamplingPreset::P444 | ChromaSubsamplingPreset::P422 => Self {
                luma_iterator: Box::new(channels.luma.iter()),
                chroma_blue_iterator: Box::new(channels.chroma_blue.iter()),
                chroma_red_iterator: Box::new(channels.chroma_red.iter()),
                subsampling_preset,
                index,
            },
            ChromaSubsamplingPreset::P420 => Self {
                luma_iterator: Box::new(ChannelIterator::new(
                    image_width,
                    &channels.luma,
                    IterationSchema::create_420_luma_schema(),
                )),
                chroma_blue_iterator: Box::new(channels.chroma_blue.iter()),
                chroma_red_iterator: Box::new(channels.chroma_red.iter()),
                subsampling_preset,
                index,
            },
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

struct ChannelIterator<'a, I> {
    image_width: usize,
    channel: &'a [CategorizedBlock],
    iteration_schema: I,
    index: usize,
}

impl<'a, I> ChannelIterator<'a, I>
where
    I: Iterator<Item = Box<dyn Iterator<Item = Movement>>>,
{
    fn new(image_width: usize, channel: &'a [CategorizedBlock], iteration_schema: I) -> Self {
        Self {
            image_width,
            channel,
            iteration_schema,
            index: 0,
        }
    }

    fn step_down(&mut self) {
        self.index += self.image_width;
    }

    fn step_right(&mut self) {
        self.index += 1;
    }

    fn step_up(&mut self) {
        self.index -= self.image_width;
    }

    fn step_left(&mut self) {
        self.index -= 1;
    }

    fn apply_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Left => self.step_left(),
            Movement::Down => self.step_down(),
            Movement::Right => self.step_right(),
            Movement::Up => self.step_up(),
        }
    }

    fn move_to_next_position(&mut self) {
        let next_movements = self.iteration_schema.next();
        for movement in next_movements.expect("Movements iterator must not end") {
            self.apply_movement(movement);
        }
    }

    fn is_out_of_range(&self) -> bool {
        self.index >= self.channel.len()
    }
}

impl<'a, T> Iterator for ChannelIterator<'a, T>
where
    T: Iterator<Item = Box<dyn Iterator<Item = Movement>>>,
{
    type Item = &'a CategorizedBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_out_of_range() {
            return None;
        }
        let return_value = &self.channel[self.index];
        self.move_to_next_position();
        Some(return_value)
    }
}

struct IterationSchema {
    index: usize,
    moves: Vec<Vec<Movement>>,
}

impl IterationSchema {
    fn create_420_luma_schema() -> Self {
        Self {
            index: 0,
            moves: vec![
                vec![Movement::Right],
                vec![Movement::Down, Movement::Left],
                vec![Movement::Right],
                vec![Movement::Up, Movement::Right],
            ],
        }
    }
}

impl Iterator for IterationSchema {
    type Item = Box<dyn Iterator<Item = Movement>>;

    fn next(&mut self) -> Option<Self::Item> {
        let return_value = Box::new(self.moves[self.index].clone().into_iter());
        self.index = (self.index + 1) % self.moves.len();
        Some(return_value)
    }
}
