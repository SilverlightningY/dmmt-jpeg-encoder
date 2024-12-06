use super::code::HuffmanCodeGenerator;
use super::coding_error::CodingError;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::VecDeque;
use std::fmt;
use std::io::Read;

#[derive(Clone, Copy)]
enum NodeKind {
    Leaf { symbol: u8 },
    OneStar { symbol: u8 },
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

impl HuffmanTree {
    pub fn new(
        symbols_and_frequencies: &[(u8, usize)],
        generator: &mut impl HuffmanCodeGenerator,
    ) -> HuffmanTree {
        let mut symbols_and_frequencies: Vec<(u8, usize)> = symbols_and_frequencies.to_vec();
        symbols_and_frequencies.sort_by(|a, b| a.1.cmp(&b.1));
        let frequencies: Vec<usize> = symbols_and_frequencies.iter().map(|a| a.1).collect();
        let code = generator.generate(&frequencies);

        let nodes: Vec<Node> = symbols_and_frequencies
            .into_iter()
            .enumerate()
            .map(|(index, (symbol, frequency))| Node {
                frequency,
                index,
                kind: NodeKind::Leaf { symbol },
            })
            .collect();

        let mut tree = HuffmanTree {
            leaf_count: nodes.len(),
            least_frequent_symbol_node_index: 0,
            nodes,
            root_index: 0,
        };

        let max_depth = code.iter().max().unwrap_or(&0).to_owned();
        let mut layers: Vec<Vec<usize>> = vec![];
        for _ in 0..=max_depth {
            layers.push(Vec::new());
        }

        for (index, &depth) in code.iter().enumerate() {
            layers[depth].push(index);
        }

        tree.build_structure(layers);

        tree
    }

    fn build_structure(&mut self, layers: Vec<Vec<usize>>) {
        // list of leafs with depths
        self.nodes.truncate(self.leaf_count);

        let mut merging_que = VecDeque::new();
        let mut future_que = VecDeque::new();

        for current_layer in layers.into_iter().rev() {
            current_layer.iter().for_each(|&i| merging_que.push_back(i));
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

    pub fn decode_sequence<I: Read>(
        &self,
        seq: &mut I,
        out: &mut Vec<u8>,
    ) -> Result<(), CodingError> {
        // tree traversal decode -> this is here for debugging not for speed
        let mut current_index = self.root_index;
        let mut buffer = [0; 1];
        let mut atbit = 0;
        let mut s = seq
            .read(&mut buffer)
            .map_err(|_| CodingError::DecoderError)?;
        while s == 1 && atbit < 8 {
            let take_right = buffer[0] & ((1 << 7) >> atbit) > 0;
            let mut node = self.nodes[current_index];
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
            node = self.nodes[current_index];
            match node.kind {
                NodeKind::Leaf { symbol } => {
                    out.push(symbol);
                    current_index = self.root_index;
                }
                NodeKind::OneStar { symbol } => {
                    out.push(symbol);
                    atbit += 1;
                    current_index = self.root_index;
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
    use crate::huffman::length_limited::LengthLimitedHuffmanCodeGenerator;

    use super::{HuffmanTree, NodeKind};

    fn calculate_depth_for_each_node(tree: &HuffmanTree) -> Vec<usize> {
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

    const SYMBOLS_AND_FREQUENCIES_EVEN_LEN: &[(u8, usize); 6] =
        &[(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12)];
    const SYMBOLS_AND_FREQUENCIES_ODD_LEN: &[(u8, usize); 7] =
        &[(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];

    #[test]
    fn test_calculate_depth_for_each_symbol_even_len() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_EVEN_LEN, &mut code_generator);
        let symbol_depths = calculate_depth_for_each_node(&tree);
        let expected_symbol_depths = [5, 5, 4, 3, 3, 3];
        for (index, (depth, expected_depth)) in symbol_depths
            .into_iter()
            .zip(expected_symbol_depths)
            .enumerate()
        {
            assert_eq!(
                depth, expected_depth,
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_odd_len() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        let symbol_depths = calculate_depth_for_each_node(&tree);
        let expected_symbol_depths = [5, 5, 4, 4, 4, 3, 3];
        for (index, (depth, expected_depth)) in symbol_depths
            .into_iter()
            .zip(expected_symbol_depths)
            .enumerate()
        {
            assert_eq!(
                depth, expected_depth,
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_calculate_depth_for_each_symbol_with_right_growing_and_onestar_pattern_replaced_tree() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        tree.replace_onestar();
        let symbol_depths = calculate_depth_for_each_node(&tree);
        let expected_symbol_depths = [6, 5, 4, 4, 4, 3, 3];
        for (index, (depth, expected_depth)) in symbol_depths
            .into_iter()
            .zip(expected_symbol_depths)
            .enumerate()
        {
            assert_eq!(
                depth, expected_depth,
                "Depth at index {} does not match",
                index
            );
        }
    }

    #[test]
    fn test_find_first_occurence_of_least_frequent_symbol_node_index() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        let expected = 0;
        let actual = tree.least_frequent_symbol_node_index;
        assert_eq!(
            expected, actual,
            "First occurence of node with the smallest frequency should be selected."
        );
    }

    #[test]
    fn test_find_first_occurence_of_least_frequent_symbol_node_index_after_onestar_pattern_replacement(
    ) {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        tree.replace_onestar();
        let expected = 0;
        let actual = tree.least_frequent_symbol_node_index;
        assert_eq!(
            expected, actual,
            "First occurence of node with the smallest frequency should be selected."
        );
    }

    #[test]
    fn test_get_max_depth_under_node() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        let depth = get_max_depth_under_node(11, &tree);
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

    fn compare_frequencies(a: &(u8, usize), b: &(u8, usize)) -> std::cmp::Ordering {
        a.1.cmp(&b.1)
    }

    fn assert_higher_frequent_symbol_has_less_depth_in_tree(
        symbols_and_frequencies: &[(u8, usize)],
        tree: &HuffmanTree,
    ) {
        let depths = calculate_depth_for_each_node(tree);
        for (depths, sf) in depths.windows(2).zip(symbols_and_frequencies.windows(2)) {
            let left_depth = depths[0];
            let right_depth = depths[1];
            let left_symbol = sf[0].0;
            let right_symbol = sf[1].0;
            let left_frequency = sf[0].1;
            let right_freqency = sf[1].1;
            assert!(
                left_depth >= right_depth,
                "Depth {} of symbol {} with frequency {} is less than depth {} of symbol {} with frequency {}",
                left_depth,
                left_symbol,
                left_frequency,
                right_depth,
                right_symbol,
                right_freqency
            )
        }
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_right_growing_tree() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut symbols_and_frequencies = *SYMBOLS_AND_FREQUENCIES_ODD_LEN;
        symbols_and_frequencies.sort_by(compare_frequencies);
        let tree = HuffmanTree::new(&symbols_and_frequencies, &mut code_generator);
        assert_higher_frequent_symbol_has_less_depth_in_tree(&symbols_and_frequencies, &tree);
    }

    #[test]
    fn test_higher_frequent_symbols_must_have_less_depth_with_right_growing_and_onestart_pattern_replaced_tree(
    ) {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut symbols_and_frequencies = *SYMBOLS_AND_FREQUENCIES_ODD_LEN;
        symbols_and_frequencies.sort_by(compare_frequencies);
        let mut tree = HuffmanTree::new(&symbols_and_frequencies, &mut code_generator);
        tree.replace_onestar();
        assert_higher_frequent_symbol_has_less_depth_in_tree(&symbols_and_frequencies, &tree);
    }

    #[test]
    fn test_each_node_has_correct_index_with_right_growing_tree() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        for (index, node) in tree.nodes.iter().enumerate() {
            assert_eq!(index, node.index);
        }
    }

    #[test]
    fn test_each_node_has_correct_index_with_right_growing_and_onestar_pattern_replaced_tree() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        tree.replace_onestar();
        for (index, node) in tree.nodes.iter().enumerate() {
            assert_eq!(index, node.index);
        }
    }

    const TEST_SYMBOL_SEQUENCE: &[u8] = &[1, 3, 2, 2, 7, 5, 4, 4, 1];
    const TEST_BYTE_SEQUENCE: &[u8] = &[0b01110111, 0b10111101, 0b00001110, 0b11100100];

    #[test]
    fn test_coder_decode() {
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let mut tree = HuffmanTree::new(SYMBOLS_AND_FREQUENCIES_ODD_LEN, &mut code_generator);
        tree.replace_onestar();
        let sequence = Vec::from(TEST_BYTE_SEQUENCE);
        let mut symbol_sequence = Vec::new();
        tree.decode_sequence(&mut sequence.as_slice(), &mut symbol_sequence)
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
        let mut code_generator = LengthLimitedHuffmanCodeGenerator::new(10);
        let tree = HuffmanTree::new(symbols_and_frequencies, &mut code_generator);
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
