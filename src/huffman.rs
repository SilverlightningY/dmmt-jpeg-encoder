use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse};
use std::collections::{BinaryHeap, VecDeque};
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
    frequency: usize,
    index: usize,
    kind: NodeKind,
}
pub struct HuffmanTree {
    nodes: Vec<Node>,
    root_index: usize,
    least_frequent_symbol_node_index: usize,
    leaf_count: usize,
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
fn replace_one_star_pattern(
    tree: &mut HuffmanTree,
    current_node_index: usize,
    only_ones_taken: bool,
) {
    let node = tree.nodes[current_node_index];
    match node.kind {
        NodeKind::Leaf { symbol: _ } => {
            if only_ones_taken {
                // switch smallest node index into this position
                tree.nodes[current_node_index].index = tree.least_frequent_symbol_node_index;
                tree.nodes[tree.least_frequent_symbol_node_index].index = current_node_index;
                tree.nodes
                    .swap(current_node_index, tree.least_frequent_symbol_node_index);
                if let NodeKind::Leaf { symbol } = tree.nodes[current_node_index].kind {
                    tree.nodes[node.index].kind = NodeKind::OneStar { symbol };
                } else {
                    panic!("Leaf with smallest frequency not a leaf?");
                }
            }
        }
        NodeKind::OneStar { symbol: _ } => (),
        NodeKind::Inner {
            left: left_node_index,
            right: right_node_index,
        } => {
            replace_one_star_pattern(tree, left_node_index, false);
            replace_one_star_pattern(tree, right_node_index, only_ones_taken);
        }
    }
}

fn merge_lists(list1: Vec<Vec<usize>>, list2: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let mut result_list = list2;
    for (pos, layer) in list1.iter().enumerate() {
        if pos < result_list.len() {
            layer.iter().for_each(|i| result_list[pos].push(*i));
        } else {
            result_list.push(layer.to_vec());
        }
    }
    let mut final_list = vec![vec![]];
    final_list.append(&mut result_list);
    final_list
}

// returns a list of leafs at each depth level of the tree
fn leaf_list(root_index: usize, tree: &HuffmanTree) -> Vec<Vec<usize>> {
    let node = tree.nodes[root_index];
    match node.kind {
        NodeKind::OneStar { symbol: _ } => {
            panic!("reordering needs to happen before replacing the onestar pattern")
        }
        NodeKind::Leaf { symbol: _ } => {
            vec![vec![node.index]]
        }
        NodeKind::Inner { left, right } => {
            let left_leaf_list = leaf_list(left, tree);
            let right_leaf_list = leaf_list(right, tree);
            merge_lists(left_leaf_list, right_leaf_list)
        }
    }
}

impl HuffmanTree {
    pub fn new(symbols_and_frequencies: &[(u32, usize)]) -> HuffmanTree {
        let mut heap = BinaryHeap::new();
        let mut nodes: Vec<Node> = vec![];

        let mut least_frequent_symbol_node_index = 0;
        let mut smallest_frequency = usize::MAX;
        let leaf_count = symbols_and_frequencies.len();
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
                least_frequent_symbol_node_index = node.index;
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
            least_frequent_symbol_node_index,
            leaf_count,
        }
    }

    pub fn reorder_right_growing(&mut self) {
        // list of leafs with depths
        let layers = leaf_list(self.root_index, self);
        self.nodes.truncate(self.leaf_count);

        let mut merging_que = VecDeque::new();
        let mut future_que = VecDeque::new();

        for current_layer in layers.into_iter().rev() {
            current_layer.iter().for_each(|i| merging_que.push_back(*i));
            while merging_que.len() > 1 {
                let right = self.nodes[merging_que.pop_front().unwrap()];
                let left = self.nodes[merging_que.pop_front().unwrap()];
                let node = Node {
                    frequency: left.frequency + right.frequency,
                    index: self.nodes.len(),
                    kind: NodeKind::Inner {
                        left: left.index,
                        right: right.index,
                    },
                };
                self.nodes.push(node);
                future_que.push_back(node.index);
            }
            merging_que.extend(future_que.iter());
            future_que.clear();
        }
        self.root_index = merging_que.pop_front().unwrap();
    }

    pub fn replace_onestar(&mut self) {
        replace_one_star_pattern(self, self.root_index, true);
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
    tree: &HuffmanTree,
    current_node_index: usize,
    current_pattern: Max32BitPattern,
) {
    let node = tree.nodes[current_node_index];
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
            fill_table(table, tree, left, current_pattern.push(false));
            fill_table(table, tree, right, current_pattern.push(true));
        }
    }
}

impl HuffmanCoder<'_> {
    pub fn new(tree: &HuffmanTree) -> HuffmanCoder {
        let mut encoding_table = Vec::new();

        fill_table(
            &mut encoding_table,
            tree,
            tree.root_index,
            Max32BitPattern::new(),
        );

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
        seq: &mut I,
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
    use std::{collections::HashMap, io::Write};

    use crate::{binary_stream::BitWriter, huffman::CodingError};

    use super::{HuffmanCoder, HuffmanTree, Max32BitPattern, NodeKind};

    fn calculate_depth_for_each_symbol(tree: &HuffmanTree) -> Vec<usize> {
        let mut return_value = vec![usize::default(); tree.nodes.len()];
        return_value[tree.root_index] = 1;
        let mut node_index_stack = vec![tree.root_index];
        while let Some(index) = node_index_stack.pop() {
            let current_node = tree.nodes[index];
            match current_node.kind {
                NodeKind::Inner {
                    left: left_node_index,
                    right: right_node_index,
                } => {
                    return_value[left_node_index] = return_value[index] + 1;
                    return_value[right_node_index] = return_value[index] + 1;
                    node_index_stack.push(left_node_index);
                    node_index_stack.push(right_node_index);
                }
                NodeKind::OneStar { symbol: _ } => {
                    return_value[index] += 1;
                }
                NodeKind::Leaf { symbol: _ } => continue,
            }
        }

        return_value
    }

    fn get_max_depth_under_node(node_index: usize, tree: &HuffmanTree) -> usize {
        get_depth_under_node(node_index, tree, &usize::max)
    }

    fn get_min_depth_under_node(node_index: usize, tree: &HuffmanTree) -> usize {
        get_depth_under_node(node_index, tree, &usize::min)
    }

    fn get_depth_under_node(
        node_index: usize,
        tree: &HuffmanTree,
        predicate: &dyn Fn(usize, usize) -> usize,
    ) -> usize {
        let root_node = tree.nodes[node_index];
        match root_node.kind {
            NodeKind::Leaf { symbol: _ } => 1,
            NodeKind::Inner { left, right } => {
                predicate(
                    get_depth_under_node(left, tree, predicate),
                    get_depth_under_node(right, tree, predicate),
                ) + 1
            }
            NodeKind::OneStar { symbol: _ } => 2,
        }
    }

    const SYMBOLS_AND_FREQUENCIES_EVEN_LEN: &[(u32, usize); 6] =
        &[(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12)];
    const SYMBOLS_AND_FREQUENCIES_ODD_LEN: &[(u32, usize); 7] =
        &[(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];

    #[test]
    fn test_calculate_depth_for_each_symbol_even_len() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_EVEN_LEN);
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 5, 3, 3, 4, 3, 2, 2, 1];
        for (index, &depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_odd_len() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 5, 3, 4, 4, 4, 3, 3, 2, 2, 1];
        for (index, &depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_with_right_growing_tree() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 5, 3, 4, 4, 4, 3, 3, 2, 2, 1];
        for (index, &depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_with_right_growing_and_onestar_pattern_replaced_tree() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let symbol_depths = calculate_depth_for_each_symbol(&tree);
        let expected_symbol_depths = [3, 5, 4, 6, 3, 4, 4, 4, 3, 3, 2, 2, 1];
        for (index, depth) in symbol_depths.iter().enumerate() {
            assert_eq!(
                *depth, expected_symbol_depths[index],
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_find_first_occurence_of_least_frequent_symbol_node_index() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        assert_eq!(
            tree.least_frequent_symbol_node_index, 1,
            "First occurence of node with the smallest frequency should be selected."
        );
    }

    #[test]
    fn test_find_first_occurence_of_least_frequent_symbol_node_index_after_onestar_pattern_replacement(
    ) {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        assert_eq!(
            tree.least_frequent_symbol_node_index, 1,
            "First occurence of node with the smallest frequency should be selected."
        );
    }

    #[test]
    fn test_get_max_depth_under_node() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        let depth = get_max_depth_under_node(10, &tree);
        assert_eq!(
            depth, 2,
            "The depth below a Inner node must be the max depth of the subtree."
        );
        let depth = get_max_depth_under_node(12, &tree);
        assert_eq!(
            depth, 5,
            "The depth below the root node must be the max depth of the tree."
        );
        let depth = get_max_depth_under_node(3, &tree);
        assert_eq!(depth, 1, "The depth below a Leaf must be 1.");
    }

    fn assert_depth_between(depths: &[usize], index: usize, less_eq: &[usize], more_eq: &[usize]) {
        for &i in less_eq {
            assert!(
                depths[index] >= depths[i],
                "Depth at index {} must be greater than depth at index {}",
                index,
                i
            );
        }
        for &i in more_eq {
            assert!(
                depths[index] <= depths[i],
                "Depth at index {} must be less or equal to depth at index {}",
                index,
                i
            );
        }
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_default_tree() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if they come later, be placed on a
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
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if they come later, be placed on a
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
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let depths = calculate_depth_for_each_symbol(&tree);
        // symbol with the same frequency as the test symbol should be placed in the more_eq slice,
        // because they can be placed at the same depth, or if they come later, be placed on a
        // higher depth.
        assert_depth_between(&depths, 0, &[4], &[1, 3, 2, 5, 6]);
        assert_depth_between(&depths, 3, &[2, 5, 6, 0, 4], &[3]);
        assert_depth_between(&depths, 2, &[6, 0, 4], &[5, 1, 3]);
        assert_depth_between(&depths, 1, &[2, 5, 6, 0, 4], &[1]);
        assert_depth_between(&depths, 4, &[], &[1, 3, 2, 5, 6, 0]);
        assert_depth_between(&depths, 5, &[6, 0, 4], &[1, 3, 2]);
        assert_depth_between(&depths, 6, &[0, 4], &[1, 3, 2, 5]);
    }

    #[test]
    fn test_each_node_has_correct_index_with_default_tree() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        for (index, node) in tree.nodes.iter().enumerate() {
            assert_eq!(index, node.index);
        }
    }

    #[test]
    fn test_each_node_has_correct_index_with_right_growing_tree() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        for (index, node) in tree.nodes.iter().enumerate() {
            assert_eq!(index, node.index);
        }
    }

    #[test]
    fn test_each_node_has_correct_index_with_right_growing_and_onestar_pattern_replaced_tree() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        for (index, node) in tree.nodes.iter().enumerate() {
            assert_eq!(index, node.index);
        }
    }

    #[test]
    fn test_pattern_push() {
        let pattern = Max32BitPattern::new();
        assert_eq!(pattern.buf, 0, "Initial buffer must be empty");
        assert_eq!(pattern.pos, 0, "Initial position must be 0");
        let pattern = pattern.push(false);
        assert_eq!(pattern.buf, 0, "After push of 0, the buffer must be 0");
        assert_eq!(pattern.pos, 1, "After push the position must be increased");
        let pattern = pattern.push(true);
        assert_eq!(
            pattern.buf, 0x40_00_00_00,
            "After push of 1, the most significant bit minus position must be set"
        );
        assert_eq!(pattern.pos, 2, "After push the position must be increased");
    }

    #[test]
    fn test_huffman_encoder_creation() {
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        let coder = HuffmanCoder::new(&tree);
        let table = coder.encoding_table;
        let expected_patterns = HashMap::from([
            (1, Max32BitPattern { buf: 0x0, pos: 2 }),
            (
                5,
                Max32BitPattern {
                    buf: 0x40_00_00_00,
                    pos: 2,
                },
            ),
            (
                2,
                Max32BitPattern {
                    buf: 0x80_00_00_00,
                    pos: 4,
                },
            ),
            (
                4,
                Max32BitPattern {
                    buf: 0x90_00_00_00,
                    pos: 4,
                },
            ),
            (
                3,
                Max32BitPattern {
                    buf: 0xA0_00_00_00,
                    pos: 3,
                },
            ),
            (
                6,
                Max32BitPattern {
                    buf: 0xC0_00_00_00,
                    pos: 3,
                },
            ),
            (
                7,
                Max32BitPattern {
                    buf: 0xE0_00_00_00,
                    pos: 3,
                },
            ),
        ]);
        for table_entry in table.iter() {
            let expected_pattern = expected_patterns.get(&table_entry.symbol).unwrap();
            let actual_pattern = table_entry.pattern;
            assert_eq!(
                actual_pattern.pos, expected_pattern.pos,
                "Pattern position does not match"
            );
            assert_eq!(
                actual_pattern.buf, expected_pattern.buf,
                "Expected pattern '{:#x}' but was '{:#x}'",
                expected_pattern.buf, actual_pattern.buf
            );
        }
    }

    const TEST_SYMBOL_SEQUENCE: &[u32] = &[1, 3, 2, 2, 7, 5, 4, 4, 1];
    const TEST_BYTE_SEQUENCE: &[u8] = &[0b00100111, 0b10111101, 0b10011110, 0b11100000];

    #[test]
    fn test_coder_encode() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let coder = HuffmanCoder::new(&tree);
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer, false);
        coder
            .encode_sequence(TEST_SYMBOL_SEQUENCE, &mut writer)
            .unwrap();
        writer.flush().expect("Flush failed");
        for (index, &byte) in buffer.iter().enumerate() {
            assert_eq!(
                byte, TEST_BYTE_SEQUENCE[index],
                "Byte at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_coder_encode_illegal_symbol() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let coder = HuffmanCoder::new(&tree);
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer, false);
        let result = coder.encode_sequence(&[1, 2, 3, 4, 5, 6, 7, 8], &mut writer);
        if let Err(CodingError::UnknownSymbolError(symbol)) = result {
            assert_eq!(symbol, 8);
        } else {
            panic!("Coding error not detected");
        }
    }

    #[test]
    fn test_coder_decode() {
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN);
        tree.reorder_right_growing();
        tree.replace_onestar();
        let coder = HuffmanCoder::new(&tree);
        let sequence = Vec::from(TEST_BYTE_SEQUENCE);
        let mut symbol_sequence = Vec::new();
        coder
            .decode_sequence(&mut sequence.as_slice(), &mut symbol_sequence)
            .unwrap();
        for (index, &symbol) in TEST_SYMBOL_SEQUENCE.iter().enumerate() {
            assert_eq!(
                symbol_sequence[index], symbol,
                "Symbol does not match at index {}",
                index
            );
        }
    }

    #[test]
    fn test_shortest_right_subtree_is_longer_eq_the_longest_left_subtree() {
        let symbols_and_frequencies = &[(1, 4), (2, 4), (3, 6), (4, 6), (5, 7), (6, 9)];
        let mut tree = HuffmanTree::new(symbols_and_frequencies);
        tree.reorder_right_growing();
        for node in &tree.nodes {
            match node.kind {
                NodeKind::Inner { left, right } => {
                    let left_max_depth = get_max_depth_under_node(left, &tree);
                    let right_min_depth = get_min_depth_under_node(right, &tree);
                    assert!(right_min_depth >= left_max_depth);
                }
                _ => continue,
            }
        }
    }
}
