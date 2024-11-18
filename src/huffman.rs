use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse};
use std::collections::BinaryHeap;
use std::fmt;
use std::io::{Read, Write};

use crate::binary_stream::BitWriter;

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
    smallest_node_index: usize,
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
fn move_longer_subtree_to_the_right(
    tree: &mut HuffmanTree,
    current_node_index: usize,
    prevent_one_star_pattern_on_right_subtree: bool,
) -> u32 {
    let node = tree.nodes[current_node_index];
    match node.kind {
        NodeKind::Leaf { symbol: _ } => {
            if prevent_one_star_pattern_on_right_subtree {
                // switch smallest node index into this position
                let smallest = tree.nodes[tree.smallest_node_index];
                tree.nodes[tree.smallest_node_index] = node;
                tree.nodes[node.index] = smallest;
                if let NodeKind::Leaf { symbol } = tree.nodes[node.index].kind {
                    tree.nodes[node.index].kind = NodeKind::OneStar { symbol };
                } else {
                    panic!("Leaf with smallest frequency not a leaf?");
                }
                return 2;
            }
            1
        }
        NodeKind::OneStar { symbol: _ } => 2,
        NodeKind::Inner {
            left: left_node_index,
            right: right_node_index,
        } => {
            let left_tree_depth = move_longer_subtree_to_the_right(tree, left_node_index, false);
            let right_tree_depth = move_longer_subtree_to_the_right(
                tree,
                right_node_index,
                prevent_one_star_pattern_on_right_subtree,
            );
            // if wrong order, swap
            if left_tree_depth > right_tree_depth {
                tree.nodes[node.index].kind = NodeKind::Inner {
                    left: right_node_index,
                    right: left_node_index,
                };
                return left_tree_depth + 1;
            }
            right_tree_depth + 1
        }
    }
}

impl HuffmanTree {
    pub fn new(symbols_and_frequencies: &[(u32, u32)]) -> HuffmanTree {
        let mut heap = BinaryHeap::new();
        let mut nodes: Vec<Node> = vec![];

        let mut smallest_node_index = 0;
        let mut smallest_frequency = 0xFFFFFFFF;
        // create the initial nodeset
        for &(symbol, frequency) in symbols_and_frequencies.iter() {
            let node = Node {
                frequency,
                index: nodes.len(),
                kind: NodeKind::Leaf { symbol },
            };
            heap.push(Reverse(node));
            nodes.push(node);
            if frequency < smallest_frequency {
                smallest_frequency = frequency;
                smallest_node_index = node.index;
            }
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
        HuffmanTree {
            nodes,
            root_index,
            smallest_node_index,
        }
    }

    pub fn reorder_right_growing(&mut self) {
        move_longer_subtree_to_the_right(self, self.root_index, false);
    }

    pub fn replace_onestar(&mut self) {
        move_longer_subtree_to_the_right(self, self.root_index, true);
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

    pub fn encode_sequence<T: Write>(
        &self,
        seq: &[u32],
        bitwriter: &mut BitWriter<T>,
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
                NodeKind::Inner { left, right } => {
                    if take_right {
                        current_index = right;
                    } else {
                        current_index = left;
                    }
                }
                _ => {
                    panic!("unreachable, only one symbol in tree")
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
                _ => {}
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

const BOX_DRAWINGS_DOUBLE_HORIZONTAL: &str = "═";
const SPACE: &str = " ";

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
                let mut result: Vec<String> = Vec::new();

                result.push(format!(
                    "{}•{}",
                    SPACE.repeat(left_width),
                    SPACE.repeat(right_width)
                ));
                result.push(format!(
                    "{}║{}",
                    SPACE.repeat(left_width),
                    SPACE.repeat(right_width)
                ));

                let left_pos = (left_box[0].chars().position(|c| c != ' ').unwrap() * 2
                    + left_box[0].trim().chars().count())
                    / 2;
                let right_pos = (right_box[0].chars().position(|c| c != ' ').unwrap() * 2
                    + right_box[0].trim().chars().count())
                    / 2;
                result.push(format!(
                    "{}╔{}╩{}╗{}",
                    SPACE.repeat(left_pos),
                    BOX_DRAWINGS_DOUBLE_HORIZONTAL.repeat(left_width - left_pos - 1),
                    BOX_DRAWINGS_DOUBLE_HORIZONTAL.repeat(right_pos),
                    SPACE.repeat(right_width - right_pos - 1)
                ));

                let left_depth = left_box.len();
                let right_depth = right_box.len();
                for i in 0..std::cmp::max(left_depth, right_depth) {
                    let mut left_str = SPACE.repeat(left_width);
                    let mut right_str = SPACE.repeat(right_width);
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

#[cfg(test)]
mod test {
    use super::{HuffmanTree, NodeKind};

    fn calculate_depth_for_each_symbol(tree: &HuffmanTree) -> Vec<usize> {
        let mut return_value = vec![usize::default(); tree.nodes.len()];
        return_value[tree.root_index] = 1;
        for _ in 0..return_value.len() {
            for (node_index, node) in tree.nodes.iter().enumerate() {
                if return_value[node_index] == usize::default() {
                    continue;
                }
                match node.kind {
                    NodeKind::Inner {
                        left: left_node_index,
                        right: right_node_index,
                    } => {
                        return_value[left_node_index] = return_value[node_index] + 1;
                        return_value[right_node_index] = return_value[node_index] + 1;
                    }
                    NodeKind::OneStar { symbol: _ } => {
                        return_value[node_index] += 1;
                    }
                    NodeKind::Leaf { symbol: _ } => continue,
                }
            }
        }

        return_value
    }

    fn get_max_depth_under_node(node_index: usize, tree: &HuffmanTree, depths: &[usize]) -> usize {
        let root_node = tree.nodes[node_index];
        match root_node.kind {
            NodeKind::Leaf { symbol: _ } => return 1,
            NodeKind::OneStar { symbol: _ } => return 2,
            _ => {}
        };
        let mut node_index_stack = vec![node_index];
        let mut max_depth = depths[node_index];
        let mut max_depth_index = node_index;
        while let Some(current_node_index) = node_index_stack.pop() {
            let current_node = tree.nodes[current_node_index];
            match current_node.kind {
                NodeKind::Leaf { symbol: _ } | NodeKind::OneStar { symbol: _ } => {
                    let current_node_depth = depths[current_node_index];
                    if current_node_depth > max_depth {
                        max_depth = current_node_depth;
                        max_depth_index = current_node_index;
                    }
                }
                NodeKind::Inner { left, right } => {
                    node_index_stack.push(left);
                    node_index_stack.push(right);
                }
            }
        }
        depths[max_depth_index] - depths[node_index] + 1
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_even_len() {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12)];
        let tree = HuffmanTree::new(&symbols_and_frequencies);
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 5, 3, 3, 4, 3, 2, 2, 1];
        for (index, depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                *depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_odd_len() {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];
        let tree = HuffmanTree::new(&symbols_and_frequencies);
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 5, 3, 4, 4, 4, 3, 3, 2, 2, 1];
        for (index, depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                *depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_get_max_depth_under_node() {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];
        let tree = HuffmanTree::new(&symbols_and_frequencies);
        let symbol_depths = [3, 5, 4, 5, 3, 4, 4, 4, 3, 3, 2, 2, 1];
        let depth = get_max_depth_under_node(10, &tree, &symbol_depths);
        assert_eq!(
            depth, 2,
            "The depth below a Inner node must be the max depth of the subtree."
        );
        let depth = get_max_depth_under_node(12, &tree, &symbol_depths);
        assert_eq!(
            depth, 5,
            "The depth below the root node must be the max depth of the tree."
        );
        let depth = get_max_depth_under_node(3, &tree, &symbol_depths);
        assert_eq!(depth, 1, "The depth below a Leaf must be 1.");
    }

    fn assert_depth_between(depths: &[usize], index: usize, less_eq: &[usize], more_eq: &[usize]) {
        for i in less_eq {
            assert!(
                depths[index] >= depths[*i],
                "Depth at index {} must be greater than depth at index {}",
                index,
                i
            );
        }
        for i in more_eq {
            assert!(
                depths[index] <= depths[*i],
                "Depth at index {} must be less or equal to depth at index {}",
                index,
                i
            );
        }
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_default_tree() {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];
        let tree = HuffmanTree::new(&symbols_and_frequencies);
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if the come later, be placed on a
        // higher depth.
        assert_depth_between(&depths, 0, &[4], &[1, 3, 2, 5, 6]);
        assert_depth_between(&depths, 1, &[2, 5, 6, 0, 4], &[3]);
        assert_depth_between(&depths, 2, &[6, 0, 4], &[5, 1, 3]);
        assert_depth_between(&depths, 3, &[2, 5, 6, 0, 4], &[1]);
        assert_depth_between(&depths, 4, &[], &[1, 3, 2, 5, 6, 0]);
        assert_depth_between(&depths, 5, &[6, 0, 4], &[1, 3, 2]);
        assert_depth_between(&depths, 6, &[0, 4], &[1, 3, 2, 5]);
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_right_growing_tree() {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];
        let mut tree = HuffmanTree::new(&symbols_and_frequencies);
        tree.reorder_right_growing();
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if the come later, be placed on a
        // higher depth.
        assert_depth_between(&depths, 0, &[4], &[1, 3, 2, 5, 6]);
        assert_depth_between(&depths, 1, &[2, 5, 6, 0, 4], &[3]);
        assert_depth_between(&depths, 2, &[6, 0, 4], &[5, 1, 3]);
        assert_depth_between(&depths, 3, &[2, 5, 6, 0, 4], &[1]);
        assert_depth_between(&depths, 4, &[], &[1, 3, 2, 5, 6, 0]);
        assert_depth_between(&depths, 5, &[6, 0, 4], &[1, 3, 2]);
        assert_depth_between(&depths, 6, &[0, 4], &[1, 3, 2, 5]);
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_right_growing_and_onestart_pattern_replaced_tree(
    ) {
        let symbols_and_frequencies = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];
        let mut tree = HuffmanTree::new(&symbols_and_frequencies);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if the come later, be placed on a
        // higher depth.
        assert_depth_between(&depths, 0, &[4], &[1, 3, 2, 5, 6]);
        assert_depth_between(&depths, 1, &[2, 5, 6, 0, 4], &[3]);
        assert_depth_between(&depths, 2, &[6, 0, 4], &[5, 1, 3]);
        assert_depth_between(&depths, 3, &[2, 5, 6, 0, 4], &[1]);
        assert_depth_between(&depths, 4, &[], &[1, 3, 2, 5, 6, 0]);
        assert_depth_between(&depths, 5, &[6, 0, 4], &[1, 3, 2]);
        assert_depth_between(&depths, 6, &[0, 4], &[1, 3, 2, 5]);
    }
}
