use std::{cmp::Reverse, collections::BinaryHeap};

use super::{HuffmanCode, HuffmanCodeGenerator};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Node {
    frequency: usize,
    kind: NodeKind,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum NodeKind {
    Package,
    Leaf,
}

pub struct LengthLimitedHuffmanCodeGenerator<'a> {
    sorted_frequencies: &'a [Reverse<Node>],
    limit: usize,
    packages: Vec<BinaryHeap<Reverse<Node>>>,
    solution: Vec<Vec<Node>>,
}

fn last_index<T>(slice: &[T]) -> usize {
    slice.len() - 1
}

impl<'a> LengthLimitedHuffmanCodeGenerator<'a> {
    fn new(
        sorted_frequencies: &'a [Reverse<Node>],
        limit: usize,
    ) -> LengthLimitedHuffmanCodeGenerator<'_> {
        assert!(
            sorted_frequencies.is_sorted(),
            "Frequencies must be sorted in descending order"
        );
        let packages = vec![BinaryHeap::new(); limit];
        let solution = vec![Vec::new(); limit];
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
            kind: NodeKind::Leaf,
        })
    }

    fn calculate_packages(&mut self) {
        self.packages[0].extend(self.sorted_frequencies);
        for level in 1..self.packages.len() {
            let previous_nodes: Vec<Node> = self.packages[level - 1]
                .clone()
                .into_sorted_vec()
                .into_iter()
                .map(|n| n.0)
                .collect();
            self.packages[level].extend(Self::merge_pairwise(&previous_nodes).map(Reverse));
            self.packages[level].extend(self.sorted_frequencies);
        }
    }

    fn get_number_of_packages_in(nodes: &[Node]) -> usize {
        nodes.iter().filter(|n| n.kind == NodeKind::Package).count()
    }

    fn calculate_initial_solution(&self) -> Vec<Node> {
        self.packages
            .last()
            .expect("Packages must contain at least one entry")
            .clone()
            .into_sorted_vec()
            .into_iter()
            .take(2 * self.sorted_frequencies.len() - 2)
            .map(|r| r.0)
            .collect()
    }

    fn calculate_solution_at(&self, level: usize) -> Vec<Node> {
        let count = Self::get_number_of_packages_in(&self.solution[level + 1]);
        self.packages[level]
            .clone()
            .into_sorted_vec()
            .into_iter()
            .take(2 * count)
            .map(|r| r.0)
            .collect()
    }

    fn calculate_solution(&mut self) {
        let last_solution_index = last_index(&self.solution);
        self.solution[last_solution_index] = self.calculate_initial_solution();
        for level in (0..last_solution_index).rev() {
            self.solution[level] = self.calculate_solution_at(level);
        }
    }

    fn get_indexes_of_leaf_nodes(nodes: &[Node]) -> impl Iterator<Item = usize> + '_ {
        nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NodeKind::Leaf)
            .map(|(index, _)| index)
    }

    fn sum_up_codeword_lengths(&mut self) -> HuffmanCode {
        self.solution
            .iter()
            .rev()
            .flat_map(|v| Self::get_indexes_of_leaf_nodes(v))
            .fold(
                vec![usize::default(); self.sorted_frequencies.len()],
                |mut v, index| {
                    v[index] += 1;
                    v
                },
            )
    }
}

impl HuffmanCodeGenerator for LengthLimitedHuffmanCodeGenerator<'_> {
    fn generate(&mut self) -> HuffmanCode {
        self.calculate_packages();
        self.calculate_solution();
        self.sum_up_codeword_lengths()
    }
}
