use std::collections::BinaryHeap;

use super::{HuffmanCode, HuffmanCodeGenerator};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum NodeKind {
    Leaf,
    Package,
}

pub struct LengthLimitedHuffmanCodeGenerator<'a> {
    sorted_frequencies: &'a [Node],
    limit: usize,
    packages: Vec<BinaryHeap<Node>>,
    solution: Vec<Vec<Node>>,
}

impl HuffmanCodeGenerator for LengthLimitedHuffmanCodeGenerator<'_> {
    fn generate(&mut self) -> HuffmanCode {
        self.calculate_packages();
        self.calculate_solution();
        self.sum_up_codeword_lengths()
    }
}

impl LengthLimitedHuffmanCodeGenerator<'_> {
    fn calculate_packages(&mut self) {
        self.packages.push(self.calculate_initial_package());
        for _ in 1..self.limit {
            self.packages.push(self.calculate_next_package());
        }
    }

    fn calculate_solution(&mut self) {
        self.solution.push(self.calculate_initial_solution());
        for _ in 1..self.limit {
            self.solution.push(self.calculate_next_solution());
        }
    }

    fn sum_up_codeword_lengths(&mut self) -> HuffmanCode {
        self.solution
            .iter()
            .flat_map(|v| 0..get_number_of_leaf_nodes_in(v))
            .fold(
                vec![usize::default(); self.sorted_frequencies.len()],
                |mut v, index| {
                    v[index] += 1;
                    v
                },
            )
    }
}

impl<'a> LengthLimitedHuffmanCodeGenerator<'a> {
    pub fn new(
        sorted_frequencies: &'a [Node],
        limit: usize,
    ) -> LengthLimitedHuffmanCodeGenerator<'_> {
        assert!(
            sorted_frequencies.is_sorted(),
            "Frequencies must be sorted in descending order"
        );
        let packages = Vec::with_capacity(limit);
        let solution = Vec::with_capacity(limit);
        LengthLimitedHuffmanCodeGenerator {
            limit,
            sorted_frequencies,
            packages,
            solution,
        }
    }

    fn merge_pairwise(nodes: &[Node]) -> impl Iterator<Item = Node> + '_ {
        nodes.chunks_exact(2).map(|s| Node {
            frequency: s[0].frequency + s[1].frequency,
            kind: NodeKind::Package,
        })
    }

    fn calculate_initial_package(&self) -> BinaryHeap<Node> {
        BinaryHeap::from_iter(self.sorted_frequencies.iter().cloned())
    }

    fn calculate_next_package(&self) -> BinaryHeap<Node> {
        let previous_nodes: Vec<Node> = self
            .packages
            .last()
            .expect("Packages must be initialized with the unmodified frequencies first")
            .clone()
            .into_sorted_vec()
            .into_iter()
            .collect();
        let mut return_value = BinaryHeap::from_iter(Self::merge_pairwise(&previous_nodes));
        return_value.extend(self.sorted_frequencies);
        return_value
    }

    fn calculate_initial_solution(&self) -> Vec<Node> {
        self.packages
            .last()
            .expect("Packages must contain at least one entry")
            .clone()
            .into_sorted_vec()
            .into_iter()
            .take(2 * self.sorted_frequencies.len() - 2)
            .collect()
    }

    fn calculate_next_solution(&self) -> Vec<Node> {
        let last_solution = self
            .solution
            .last()
            .expect("Solution must be initialized first.");
        let count = get_number_of_package_nodes_in(last_solution);
        let level = self.packages.len() - self.solution.len() - 1;
        self.packages[level]
            .clone()
            .into_sorted_vec()
            .into_iter()
            .take(2 * count)
            .collect()
    }
}

fn get_number_of_package_nodes_in(nodes: &[Node]) -> usize {
    nodes.iter().filter(|n| n.kind == NodeKind::Package).count()
}

fn get_number_of_leaf_nodes_in(nodes: &[Node]) -> usize {
    nodes.iter().filter(|n| n.kind == NodeKind::Leaf).count()
}

#[cfg(test)]
mod test {
    use crate::huffman::HuffmanCodeGenerator;

    use super::{
        get_number_of_leaf_nodes_in, get_number_of_package_nodes_in,
        LengthLimitedHuffmanCodeGenerator, Node, NodeKind,
    };

    #[test]
    fn test_get_number_of_package_nodes_in() {
        let nodes = &[
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
        ];
        let expected = 3;
        let actual = get_number_of_package_nodes_in(nodes);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_get_number_of_leaf_nodes_in() {
        let nodes = &[
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Package,
            },
            Node {
                frequency: 7,
                kind: NodeKind::Leaf,
            },
        ];
        let expected = 4;
        let actual = get_number_of_leaf_nodes_in(nodes);
        assert_eq!(expected, actual);
    }

    fn get_test_sorted_frequencies() -> [Node; 11] {
        [
            Node {
                frequency: 1,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 2,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 5,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 8,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 10,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 11,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 14,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 14,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 15,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 18,
                kind: NodeKind::Leaf,
            },
            Node {
                frequency: 20,
                kind: NodeKind::Leaf,
            },
        ]
    }

    #[test]
    fn test_calculate_initial_package() {
        let sorted_frequencies = get_test_sorted_frequencies();
        let generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, 3);
        let initial_package = generator.calculate_initial_package();
        assert_eq!(
            initial_package.len(),
            11,
            "Lenght of initial_package does not match"
        );
        assert_eq!(
            initial_package
                .iter()
                .filter(|n| n.kind == NodeKind::Leaf)
                .count(),
            initial_package.len(),
            "Initial Nodes must all be Leafs"
        );
    }

    #[test]
    fn test_calculate_packages() {
        let limit = 4;
        let sorted_frequencies = get_test_sorted_frequencies();
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, limit);
        generator.calculate_packages();
        assert_eq!(
            generator.packages.len(),
            limit,
            "The length of the packages vector must be equal to the limit"
        );
        for (index, package) in generator.packages.iter().enumerate().skip(1) {
            assert!(
                !package.is_empty(),
                "Package at index {} must not be empty",
                index
            );
            let number_of_packages = package
                .iter()
                .filter(|n| n.kind == NodeKind::Package)
                .count();
            let expected_number_of_packages = generator.packages[index - 1].len() / 2;
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
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, limit);
        generator.calculate_packages();
        generator.calculate_solution();
        assert_eq!(
            generator.solution.len(),
            limit,
            "The length of the solution vector must be equal to the limit"
        );
        for (index, solution) in generator.solution.iter().enumerate().skip(1) {
            assert!(
                !solution.is_empty(),
                "Solution at index {} must not be empty",
                index
            );
            assert!(
                solution.is_sorted(),
                "Solution at index {} must be sorted",
                index
            );
            let expected_number_of_nodes = generator.solution[index - 1]
                .iter()
                .filter(|n| n.kind == NodeKind::Package)
                .count()
                * 2;
            assert_eq!(
                solution.len(),
                expected_number_of_nodes,
                "Number of nodes does not match at index {}",
                index
            );
        }
    }

    #[test]
    fn test_generate_one() {
        let limit = 4;
        let sorted_frequencies: [Node; 11] =
            [1, 2, 5, 8, 10, 11, 14, 14, 15, 18, 20].map(Node::from);
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, limit);
        let expected_code = [4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3];
        let code = generator.generate();
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
        let sorted_frequencies: [Node; 10] = [1, 1, 1, 2, 2, 2, 3, 6, 17, 20].map(Node::from);
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, limit);
        let expected_code = [5, 5, 4, 4, 4, 4, 4, 3, 2, 2];
        let code = generator.generate();
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
        let sorted_frequencies: [Node; 10] = [1, 1, 1, 2, 2, 2, 3, 6, 17, 20].map(Node::from);
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(&sorted_frequencies, limit);
        let expected_code = [4, 4, 4, 4, 4, 4, 4, 4, 2, 2];
        let code = generator.generate();
        for (index, (acutal_len, expected_len)) in code.into_iter().zip(expected_code).enumerate() {
            assert_eq!(
                acutal_len, expected_len,
                "Codeword lengths do not equal at index {}",
                index
            );
        }
    }
}
