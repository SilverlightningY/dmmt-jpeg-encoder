use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse};
use std::collections::BinaryHeap;
use std::fmt;
use std::io::Read;
use std::iter::repeat;

#[derive(Clone, Copy)]
enum NodeKind {
    Leaf { symbol: u32 },
    OneStar { symbol: u32 },
    Inner { left: usize, right: usize },
}

#[derive(Clone, Copy)]
struct Node {
    frequency: u32,
    index: usize,
    kind: NodeKind,
}
pub struct HuffmanTree {
    nodes: Vec<Node>,
    root_index: usize,
}

#[derive(Clone, Copy)]
struct Max32BitPattern {
    buf: u32,
    pos: usize,
}

#[derive(Clone, Copy)]
struct TableEntry {
    pattern: Max32BitPattern,
    symbol: u32,
}

#[derive(Debug)]
pub enum CodingError {
    UnknownSymbolError(u32),
    BitWriterError(std::io::Error),
    DecoderError,
}

pub struct HuffmanCoder<'a> {
    // this is sorted according to symbol
    encoding_table: Vec<TableEntry>,
    tree: &'a HuffmanTree,
}

// this function swaps the subtrees at each node
// -> the subtree with more depth is always the right child after this call
// -> if called with right_path=true it will also replace the 1* pattern
// WARNING: replacing 1* has to take place AFTER swapping (two separate calls)
fn correct_node(node: Node, tree: &mut HuffmanTree, right_path: bool) -> u32 {
    match node.kind {
        NodeKind::Leaf { symbol } => {
            if right_path {
                tree.nodes[node.index].kind = NodeKind::OneStar { symbol };
                return 2;
            }
            1
        }
        NodeKind::OneStar { symbol: _ } => 2,
        NodeKind::Inner { left, right } => {
            let res_left = correct_node(tree.nodes[left], tree, false);
            let res_right = correct_node(tree.nodes[right], tree, right_path);
            // if wrong order, swap
            if res_left > res_right {
                tree.nodes[node.index].kind = NodeKind::Inner {
                    left: right,
                    right: left,
                };
                return res_left + 1;
            }
            res_right + 1
        }
    }
}

impl HuffmanTree {
    pub fn new(symbols_and_frequencies: &[(u32, u32)]) -> HuffmanTree {
        let mut heap = BinaryHeap::new();
        let mut nodes: Vec<Node> = vec![];
        // create the initial nodeset
        for fs in symbols_and_frequencies.iter() {
            let frequency = fs.1;
            let symbol = fs.0;
            let node = Node {
                frequency,
                index: nodes.len(),
                kind: NodeKind::Leaf { symbol },
            };
            heap.push(Reverse(node));
            nodes.push(node);
        }
        // merge nodes until none left
        while heap.len() > 1 {
            let t1 = heap.pop().unwrap().0;
            let t2 = heap.pop().unwrap().0;
            let node = Node {
                frequency: t1.frequency + t2.frequency,
                index: nodes.len(),
                kind: NodeKind::Inner {
                    left: t1.index,
                    right: t2.index,
                },
            };
            heap.push(Reverse(node));
            nodes.push(node);
        }
        let root_index = heap.pop().unwrap().0.index;
        HuffmanTree { nodes, root_index }
    }

    pub fn correct_ordering(&mut self) {
        correct_node(self.nodes[self.root_index], self, false);
    }
    pub fn replace_onestar(&mut self) {
        correct_node(self.nodes[self.root_index], self, true);
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.frequency.cmp(&other.frequency)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.frequency == other.frequency
    }
}

impl Eq for Node {}

impl Max32BitPattern {
    pub fn new() -> Max32BitPattern {
        Max32BitPattern { buf: 0, pos: 0 }
    }
    pub fn push(self, val: bool) -> Max32BitPattern {
        let mut res = self;
        if res.pos >= 32 {
            panic!("In Max32BitPattern: attempted to push further than 32 bits");
        }
        if val {
            res.buf |= (1 << 31) >> res.pos;
        }
        res.pos += 1;
        res
    }
}

fn fill_table(
    table: &mut Vec<TableEntry>,
    node: Node,
    tree: &HuffmanTree,
    current_pattern: Max32BitPattern,
) {
    match node.kind {
        NodeKind::Leaf { symbol } => {
            table.push(TableEntry {
                pattern: current_pattern,
                symbol,
            });
        }
        NodeKind::OneStar { symbol } => {
            let p = current_pattern.push(false);
            table.push(TableEntry { pattern: p, symbol });
        }
        NodeKind::Inner { left, right } => {
            let left_node = tree.nodes[left];
            let right_node = tree.nodes[right];
            fill_table(table, left_node, tree, current_pattern.push(false));
            fill_table(table, right_node, tree, current_pattern.push(true));
        }
    }
}

impl HuffmanCoder<'_> {
    pub fn new(tree: &HuffmanTree) -> HuffmanCoder {
        let mut table = vec![];

        fill_table(
            &mut table,
            tree.nodes[tree.root_index],
            tree,
            Max32BitPattern::new(),
        );

        let mut encoding_table = table.clone();
        encoding_table.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        HuffmanCoder {
            encoding_table,
            tree,
        }
    }

    pub fn encode_sequence<T: std::io::Write>(
        &self,
        seq: &[u32],
        bitwriter: &mut super::binary_stream::BitWriter<T>,
    ) -> Result<(), CodingError> {
        for s in seq.iter() {
            let table_entry: TableEntry = self.encoding_table[self
                .encoding_table
                .binary_search_by(|probe| probe.symbol.cmp(s))
                .map_err(|_| CodingError::UnknownSymbolError(*s))?];
            bitwriter
                .write_bits(
                    &table_entry.pattern.buf.to_be_bytes(),
                    table_entry.pattern.pos,
                )
                .map_err(CodingError::BitWriterError)?;
        }
        Ok(())
    }

    pub fn decode_sequence<I: Read>(
        &self,
        mut seq: I,
        out: &mut Vec<u32>,
    ) -> Result<(), CodingError> {
        // tree traversal decode -> this is here for debugging not for speed
        let mut current_index = self.tree.root_index;
        let mut buffer = [0; 1];
        let mut atbit = 0;
        let mut s = seq
            .read(&mut buffer)
            .map_err(|_| CodingError::DecoderError)?;
        while s == 1 && atbit < 8 {
            let take_right = buffer[0] & ((1 << 7) >> atbit) > 0;
            let mut node = self.tree.nodes[current_index];
            match node.kind {
                NodeKind::Leaf { symbol: _ } => {
                    panic!("this branch is not reachable");
                }
                NodeKind::OneStar { symbol: _ } => {
                    panic!("this branch is not reachable");
                }
                NodeKind::Inner { left, right } => {
                    if take_right {
                        current_index = right;
                    } else {
                        current_index = left;
                    }
                }
            };
            node = self.tree.nodes[current_index];
            match node.kind {
                NodeKind::Leaf { symbol } => {
                    out.push(symbol);
                    current_index = self.tree.root_index;
                }
                NodeKind::OneStar { symbol } => {
                    out.push(symbol);
                    atbit += 1;
                    current_index = self.tree.root_index;
                }
                NodeKind::Inner { left: _, right: _ } => {}
            };
            atbit += 1;
            if atbit >= 8 {
                s = seq
                    .read(&mut buffer)
                    .map_err(|_| CodingError::DecoderError)?;
                atbit %= 8;
            }
        }
        Ok(())
    }
}

// Node & Tree visualization
impl Node {
    fn get_string(&self, tree: &HuffmanTree) -> Vec<String> {
        match self.kind {
            NodeKind::Leaf { symbol } => vec![format!("(s:{},f:{})", symbol, self.frequency)],
            NodeKind::OneStar { symbol } => {
                vec![
                    " •".to_string(),
                    " ║".to_string(),
                    "╔╝".to_string(),
                    format!("(s:{},f:{})", symbol, self.frequency),
                ]
            }
            NodeKind::Inner { left, right } => {
                let node_left = tree.nodes[left];
                let node_right = tree.nodes[right];
                let left_box: Vec<String> = node_left.get_string(tree);
                let right_box: Vec<String> = node_right.get_string(tree);
                let left_width = left_box[0].chars().count();
                let right_width = right_box[0].chars().count();
                let mut result: Vec<String> = vec![];

                let x_str = |x, s| repeat(s).take(x).collect::<String>();
                result.push(format!(
                    "{}•{}",
                    x_str(left_width, " "),
                    x_str(right_width, " ")
                ));
                result.push(format!(
                    "{}║{}",
                    x_str(left_width, " "),
                    x_str(right_width, " ")
                ));

                let left_pos = (left_box[0].chars().position(|c| c != ' ').unwrap() * 2
                    + left_box[0].trim().chars().count())
                    / 2;
                let right_pos = (right_box[0].chars().position(|c| c != ' ').unwrap() * 2
                    + right_box[0].trim().chars().count())
                    / 2;
                result.push(format!(
                    "{}╔{}╩{}╗{}",
                    x_str(left_pos, " "),
                    x_str(left_width - left_pos - 1, "═"),
                    x_str(right_pos, "═"),
                    x_str(right_width - right_pos - 1, " ")
                ));

                let left_depth = left_box.len();
                let right_depth = right_box.len();
                for i in 0..std::cmp::max(left_depth, right_depth) {
                    let mut left_str = x_str(left_width, " ");
                    let mut right_str = x_str(right_width, " ");
                    if i < left_depth {
                        left_str = left_box[i].clone();
                    }
                    if i < right_depth {
                        right_str = right_box[i].clone();
                    }
                    result.push(format!("{} {}", left_str, right_str));
                }
                result
            }
        }
    }
}

impl fmt::Display for HuffmanTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let strs = self.nodes[self.root_index].get_string(self);
        for s in strs.iter() {
            writeln!(f, "{}", s)?;
        }
        Ok(())
    }
}
