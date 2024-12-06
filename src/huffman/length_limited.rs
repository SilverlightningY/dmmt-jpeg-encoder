use std::collections::BinaryHeap;
use std::iter;

use super::code::HuffmanCode;
use super::code::HuffmanCodeGenerator;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Node {
    frequency: usize,
    kind: NodeKind,
}

impl From<usize> for Node {
    fn from(value: usize) -> Self {
        Self {
            frequency: value,
            kind: NodeKind::Leaf,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum NodeKind {
    Leaf,
    Package,
}

struct Solution {
    number_of_packages: usize,
    number_of_leafs_in_package: usize,
}

pub struct LengthLimitedHuffmanCodeGenerator {
    limit: usize,
}

impl HuffmanCodeGenerator for LengthLimitedHuffmanCodeGenerator {
    fn generate(&mut self, sorted_frequencies: &[usize]) -> HuffmanCode {
        assert!(
            sorted_frequencies.is_sorted(),
            "Frequencies must be sorted in ascending order"
        );
        let code_length = sorted_frequencies.len();
        assert!(
            code_length <= 2_usize.pow(self.limit as u32),
            "Tree of depth limit {} can not hold {} code words",
            self.limit,
            code_length
        );
        let sorted_frequencies: Vec<Node> =
            sorted_frequencies.iter().copied().map(Node::from).collect();
        let packages = Self::calculate_packages(self.limit, &sorted_frequencies);
        let solution_lengths = Self::calculate_solution(&packages, code_length);
        Self::sum_up_codeword_lengths(solution_lengths, code_length)
    }
}

impl LengthLimitedHuffmanCodeGenerator {
    pub fn new(limit: usize) -> LengthLimitedHuffmanCodeGenerator {
        LengthLimitedHuffmanCodeGenerator { limit }
    }

    fn calculate_packages(limit: usize, sorted_frequencies: &[Node]) -> Vec<Vec<Node>> {
        let initial_item = iter::once(Vec::from(sorted_frequencies));
        let following_items =
            (1..limit).scan(Vec::from(sorted_frequencies), |previous_nodes, _| {
                let next_nodes = Self::calculate_next_package(previous_nodes, sorted_frequencies);
                previous_nodes.clear();
                previous_nodes.extend(&next_nodes);
                Some(next_nodes)
            });
        initial_item.chain(following_items).collect()
    }

    fn calculate_solution(
        packages: &[Vec<Node>],
        code_length: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        let initial_solution = Solution {
            number_of_packages: code_length - 1,
            number_of_leafs_in_package: 0,
        };
        packages.iter().rev().scan(initial_solution, |s, p| {
            let next_solution = Self::calculate_next_solution(s, p);
            s.number_of_packages = next_solution.number_of_packages;
            s.number_of_leafs_in_package = next_solution.number_of_leafs_in_package;
            Some(next_solution.number_of_leafs_in_package)
        })
    }

    fn sum_up_codeword_lengths(
        solution_lengths: impl Iterator<Item = usize>,
        code_length: usize,
    ) -> HuffmanCode {
        solution_lengths.flat_map(|l| 0..l).fold(
            vec![usize::default(); code_length],
            |mut v, index| {
                v[index] += 1;
                v
            },
        )
    }

    fn merge_pairwise(nodes: &[Node]) -> impl Iterator<Item = Node> + '_ {
        nodes.chunks_exact(2).map(|s| Node {
            frequency: s[0].frequency + s[1].frequency,
            kind: NodeKind::Package,
        })
    }

    fn calculate_next_package(previous_nodes: &[Node], sorted_frequencies: &[Node]) -> Vec<Node> {
        let mut return_value = BinaryHeap::from_iter(Self::merge_pairwise(previous_nodes));
        return_value.extend(sorted_frequencies);
        return_value.into_sorted_vec()
    }

    fn calculate_next_solution(previous_solution: &Solution, package: &[Node]) -> Solution {
        let count = previous_solution.number_of_packages * 2;
        let range = &package[0..count];
        range.iter().fold(
            Solution {
                number_of_packages: 0,
                number_of_leafs_in_package: 0,
            },
            |mut solution, node| {
                match node.kind {
                    NodeKind::Leaf => solution.number_of_leafs_in_package += 1,
                    NodeKind::Package => solution.number_of_packages += 1,
                }
                solution
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::HuffmanCodeGenerator;

    use super::{LengthLimitedHuffmanCodeGenerator, Node, NodeKind};

    fn get_test_sorted_frequencies() -> [Node; 11] {
        [1, 2, 5, 8, 10, 11, 14, 14, 15, 18, 20].map(Node::from)
    }

    #[test]
    fn test_calculate_packages() {
        let limit = 4;
        let sorted_frequencies = get_test_sorted_frequencies();
        let packages =
            LengthLimitedHuffmanCodeGenerator::calculate_packages(limit, &sorted_frequencies);
        assert_eq!(
            packages.len(),
            limit,
            "The length of the packages vector must be equal to the limit"
        );
        for (index, package) in packages.iter().enumerate().skip(1) {
            println!("{:#?}", package);
            assert!(
                !package.is_empty(),
                "Package at index {} must not be empty",
                index
            );
            let number_of_packages = package
                .iter()
                .filter(|n| n.kind == NodeKind::Package)
                .count();
            let expected_number_of_packages = packages[index - 1].len() / 2;
            assert_eq!(
                number_of_packages, expected_number_of_packages,
                "Number of packages does not match at index {}",
                index
            );
            let expected_number_of_nodes = expected_number_of_packages + sorted_frequencies.len();
            assert_eq!(
                package.len(),
                expected_number_of_nodes,
                "Number of nodes does not match at index {}",
                index
            );
        }
    }

    #[test]
    fn test_calculate_solution() {
        let limit = 4;
        let sorted_frequencies = get_test_sorted_frequencies();
        let code_length = sorted_frequencies.len();
        let packages =
            LengthLimitedHuffmanCodeGenerator::calculate_packages(limit, &sorted_frequencies);
        let mut solution =
            LengthLimitedHuffmanCodeGenerator::calculate_solution(&packages, code_length);
        assert_eq!(
            solution.by_ref().count(),
            limit,
            "The length of the solution must be equal to the limit"
        );
        for (index, solution_length) in solution.by_ref().enumerate() {
            assert!(
                solution_length <= code_length,
                "Solution length at index {} must be less or equal to {}, but was {}",
                index,
                code_length,
                solution_length
            );
        }
    }

    #[test]
    fn test_generate_one() {
        let limit = 4;
        let sorted_frequencies: [usize; 11] = [1, 2, 5, 8, 10, 11, 14, 14, 15, 18, 20];
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(limit);
        let expected_code = [4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3];
        let code = generator.generate(&sorted_frequencies);
        for (index, (actual_len, expected_len)) in code.into_iter().zip(expected_code).enumerate() {
            assert_eq!(
                actual_len, expected_len,
                "Codeword lengths do not equal at index {}",
                index
            );
        }
    }

    #[test]
    fn test_generate_two() {
        let limit = 5;
        let sorted_frequencies: [usize; 10] = [1, 1, 1, 2, 2, 2, 3, 6, 17, 20];
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(limit);
        let expected_code = [5, 5, 4, 4, 4, 4, 4, 3, 2, 2];
        let code = generator.generate(&sorted_frequencies);
        for (index, (acutal_len, expected_len)) in code.into_iter().zip(expected_code).enumerate() {
            assert_eq!(
                acutal_len, expected_len,
                "Codeword lengths do not equal at index {}",
                index
            );
        }
    }

    #[test]
    fn test_generate_three() {
        let limit = 4;
        let sorted_frequencies: [usize; 10] = [1, 1, 1, 2, 2, 2, 3, 6, 17, 20];
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(limit);
        let expected_code = [4, 4, 4, 4, 4, 4, 4, 4, 2, 2];
        let code = generator.generate(&sorted_frequencies);
        for (index, (acutal_len, expected_len)) in code.into_iter().zip(expected_code).enumerate() {
            assert_eq!(
                acutal_len, expected_len,
                "Codeword lengths do not equal at index {}",
                index
            );
        }
    }

    #[test]
    #[should_panic]
    fn test_generate_too_long_input_array() {
        let limit = 3;
        let sorted_frequencies: [usize; 10] = [1, 1, 1, 2, 2, 2, 3, 6, 17, 20];
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(limit);
        let _ = generator.generate(&sorted_frequencies);
    }

    #[test]
    fn test_merge_pairwise_odd_length_list() {
        let nodes = [1, 2, 3, 4, 5, 6, 7].map(Node::from);
        let mut pair_iter = LengthLimitedHuffmanCodeGenerator::merge_pairwise(&nodes);
        assert_eq!(
            pair_iter.by_ref().count(),
            nodes.len() / 2,
            "Length of pairs must be half the lenght of the input slice"
        );
        let expected_nodes = [
            Node {
                frequency: 3,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 11,
                kind: NodeKind::Package,
            },
        ];
        for (index, (expected_node, actual_node)) in
            expected_nodes.iter().zip(pair_iter).enumerate()
        {
            assert_eq!(
                expected_node.frequency, actual_node.frequency,
                "Frequency does not match at index {}",
                index
            );
            assert!(
                actual_node.kind == NodeKind::Package,
                "Node kind must be package after merge, but was not at index {}",
                index
            );
        }
    }

    #[test]
    fn test_calculate_next_package() {
        let previous_nodes = [1, 2, 3, 4, 5].map(Node::from);
        let sorted_frequencies = [1, 3, 5, 7].map(Node::from);
        let package = LengthLimitedHuffmanCodeGenerator::calculate_next_package(
            &previous_nodes,
            &sorted_frequencies,
        );
        let number_of_packages = package
            .iter()
            .filter(|n| n.kind == NodeKind::Package)
            .count();
        let expected_number_of_packages = 2;
        assert_eq!(
            number_of_packages, expected_number_of_packages,
            "Unexpected number of package nodes in package vector"
        );
        let expected_len = previous_nodes.len() / 2 + sorted_frequencies.len();
        assert_eq!(
            package.len(),
            expected_len,
            "Unexpected length of package vector"
        );
        assert!(package.is_sorted(), "Package vector must be sorted");
    }
}
