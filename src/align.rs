use std::cmp::max;
use std::collections::VecDeque;

const DELETION_COST: usize = 2;
const INSERTION_COST: usize = 2;
const INITIAL_MISMATCH_PENALTY: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    NoOp,
    Deletion,
    Insertion,
}

use Operation::*;

#[derive(Clone, Debug)]
struct Cell {
    parent: usize,
    operation: Operation,
    cost: usize,
}

#[derive(Debug)]
pub struct Alignment<'a> {
    pub x: Vec<&'a str>,
    pub y: Vec<&'a str>,
    table: Vec<Cell>,
    dim: [usize; 2],
}

impl<'a> Alignment<'a> {
    pub fn new(x: Vec<&'a str>, y: Vec<&'a str>) -> Self {
        let dim = [y.len() + 1, x.len() + 1];
        let table = vec![
            Cell {
                parent: 0,
                operation: NoOp,
                cost: 0
            };
            dim[0] * dim[1]
        ];
        let mut alignment = Self { x, y, table, dim };
        alignment.fill();
        alignment
    }

    pub fn fill(&mut self) {
        for i in 1..self.dim[1] {
            self.table[i] = Cell {
                parent: 0,
                operation: Deletion,
                cost: i * DELETION_COST + INITIAL_MISMATCH_PENALTY,
            };
        }
        for j in 1..self.dim[0] {
            self.table[j * self.dim[1]] = Cell {
                parent: 0,
                operation: Insertion,
                cost: j * INSERTION_COST + INITIAL_MISMATCH_PENALTY,
            };
        }

        for (i, x_i) in self.x.iter().enumerate() {
            for (j, y_j) in self.y.iter().enumerate() {
                let (left, diag, up) =
                    (self.index(i, j + 1), self.index(i, j), self.index(i + 1, j));
                let candidates = [
                    Cell {
                        parent: up,
                        operation: Insertion,
                        cost: self.mismatch_cost(up, INSERTION_COST),
                    },
                    Cell {
                        parent: left,
                        operation: Deletion,
                        cost: self.mismatch_cost(left, DELETION_COST),
                    },
                    Cell {
                        parent: diag,
                        operation: NoOp,
                        cost: if x_i == y_j {
                            self.table[diag].cost
                        } else {
                            usize::MAX
                        },
                    },
                ];
                let index = self.index(i + 1, j + 1);
                self.table[index] = candidates
                    .iter()
                    .min_by_key(|cell| cell.cost)
                    .unwrap()
                    .clone();
            }
        }
    }

    fn mismatch_cost(&self, parent: usize, basic_cost: usize) -> usize {
        self.table[parent].cost
            + basic_cost
            + if self.table[parent].operation == NoOp {
                INITIAL_MISMATCH_PENALTY
            } else {
                0
            }
    }

    pub fn operations(&self) -> Vec<Operation> {
        let mut ops = VecDeque::with_capacity(max(self.x.len(), self.y.len()));
        let mut cell = &self.table[self.index(self.x.len(), self.y.len())];
        loop {
            ops.push_front(cell.operation);
            if cell.parent == 0 {
                break;
            }
            cell = &self.table[cell.parent];
        }
        Vec::from(ops)
    }

    pub fn coalesced_operations(&self) -> Vec<(Operation, usize)> {
        run_length_encode(self.operations())
    }

    fn index(&self, i: usize, j: usize) -> usize {
        j * self.dim[1] + i
    }
}

fn run_length_encode<T>(sequence: Vec<T>) -> Vec<(T, usize)>
where
    T: Copy,
    T: PartialEq,
{
    let mut encoded = Vec::with_capacity(sequence.len());

    if sequence.is_empty() {
        return encoded;
    }

    let end = sequence.len();
    let (mut i, mut j) = (0, 1);
    let mut curr = &sequence[i];
    loop {
        if j == end || sequence[j] != *curr {
            encoded.push((*curr, j - i));
            if j == end {
                return encoded;
            } else {
                curr = &sequence[j];
                i = j;
            }
        }
        j += 1;
    }
}