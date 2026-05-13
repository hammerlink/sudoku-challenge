use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::slice::Iter;

use rayon::prelude::*;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB read buffer
const ALL_OPTIONS: u16 = 511;

macro_rules! box_index {
    ($row:expr, $col:expr) => {
        ($row / 3) * 3 + ($col / 3)
    };
}

fn main() -> io::Result<()> {
    let file = File::open("sudoku.csv")?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let lines: Vec<String> = reader
        .lines()
        .map(|l| l.expect("read error"))
        .skip_while(|l| l.starts_with("puzzle"))
        .collect();

    let (complete, incomplete, incorrect) = lines
        .par_iter()
        .fold(
            || (0u64, 0u64, 0u64),
            |mut acc, line| {
                if let Some(result) = process_line(line) {
                    match result {
                        SolveState::Correct => acc.0 += 1,
                        SolveState::Incomplete => acc.1 += 1,
                        SolveState::Incorrect => acc.2 += 1,
                    }
                }
                acc
            },
        )
        .reduce(|| (0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));

    let count = complete + incomplete + incorrect;
    eprintln!("Processed {count} puzzles");
    let completed_percent = complete as f64 / count as f64 * 100.0f64;
    println!("Completed {complete} {completed_percent}");
    let incompleted_percent = incomplete as f64 / count as f64 * 100.0f64;
    println!("Incompleted {incomplete} {incompleted_percent}");
    let incorrect_percent = incorrect as f64 / count as f64 * 100.0f64;
    println!("Incorrect {incorrect} {incorrect_percent}");
    Ok(())
}

#[derive(PartialEq, Debug)]
enum SolveState {
    Correct,
    Incorrect,
    Incomplete,
}

#[derive(Copy, Clone)]
struct Possibles {
    options: u16,
}
impl Possibles {
    fn default() -> Self {
        Self {
            options: ALL_OPTIONS,
        }
    }

    fn from(value: u8) -> Self {
        if value == 0 {
            return Self {
                options: ALL_OPTIONS,
            };
        }
        Self {
            options: 1 << (value - 1),
        }
    }

    fn get_solution(&self) -> Option<usize> {
        if self.options.count_ones() != 1 {
            return None;
        }
        Some(self.options.trailing_zeros() as usize + 1)
    }
}

struct Solution {
    pub raster: [[u8; 9]; 9],
}
impl Solution {
    pub fn from(input: &str) -> Self {
        let bytes = input.as_bytes();
        let mut raster = [[0u8; 9]; 9];
        for row in 0..9 {
            for col in 0..9 {
                let val = bytes[row * 9 + col] - b'0';
                raster[row][col] = val;
            }
        }
        Self { raster }
    }
}

struct Sudoku {
    pub raster: [[u8; 9]; 9],
    /// Which row indeces are not completely filled yet
    /// 0_0000_1001 => 4th and 1st row contain missing values
    pub unsolved_rows: u16,
    /// Which columns have unknowns, similar to unsolved_rows
    pub unsolved_columns: u16,
    /// What options are still open for each row
    /// index-0: 0_0001_0010 => first row has 2 and 5 missing
    pub rows_options: [u16; 9],
    /// What options are still open for each column
    /// index-0: 0_0001_0010 => first column has 2 and 5 missing
    pub columns_options: [u16; 9],
    pub inner_box_options: [u16; 9],
    pub possibles_raster: [[Possibles; 9]; 9],
}

#[allow(unused)]
impl Sudoku {
    pub fn from(input: &str) -> Self {
        let bytes = input.as_bytes();
        let mut raster = [[0u8; 9]; 9];
        let mut possibles_raster: [[Possibles; 9]; 9] = [[Possibles::default(); 9]; 9];
        let mut rows_options = [ALL_OPTIONS; 9];
        let mut columns_options = [ALL_OPTIONS; 9];
        let mut inner_box_options = [ALL_OPTIONS; 9];
        for row in 0..9 {
            for col in 0..9 {
                let val = bytes[row * 9 + col] - b'0';
                raster[row][col] = val;

                // Filled in value, remove options
                if val > 0 {
                    possibles_raster[row][col] = Possibles::from(val);
                    rows_options[row] &= ALL_OPTIONS ^ 1 << (val - 1);
                    columns_options[col] &= ALL_OPTIONS ^ 1 << (val - 1);
                    inner_box_options[box_index!(row, col)] &= ALL_OPTIONS ^ 1 << (val - 1);
                }
            }
        }
        let unsolved_rows: u16 =
            rows_options
                .iter()
                .enumerate()
                .fold(0, |mut result, (i, value)| {
                    if *value > 0 {
                        result |= 1u16 << i;
                    }
                    result
                });
        let unsolved_columns: u16 =
            columns_options
                .iter()
                .enumerate()
                .fold(0, |mut result, (i, value)| {
                    if *value > 0 {
                        result |= 1u16 << i;
                    }
                    result
                });
        Self {
            raster,
            unsolved_rows,
            unsolved_columns,
            rows_options,
            columns_options,
            inner_box_options,
            possibles_raster,
        }
    }

    pub fn set_value(&mut self, row: usize, col: usize, value: u8) {
        if self.raster[row][col] > 0 {
            panic!("already assigned {row} {col}");
        }
        self.raster[row][col] = value;
        self.possibles_raster[row][col].options = 1 << (value - 1);
        self.rows_options[row] &= ALL_OPTIONS ^ 1 << (value - 1);
        self.columns_options[col] &= ALL_OPTIONS ^ 1 << (value - 1);
        self.inner_box_options[box_index!(row, col)] &= ALL_OPTIONS ^ 1 << (value - 1);
        if self.rows_options[row] == 0 {
            self.unsolved_rows &= ALL_OPTIONS ^ 1 << row;
        }
        if self.columns_options[col] == 0 {
            self.unsolved_columns &= ALL_OPTIONS ^ 1 << col;
        }
    }

    pub fn iter_row(&self, row: usize) -> Iter<'_, u8> {
        self.raster[row].iter()
    }

    pub fn iter_column(&self, column: usize) -> impl Iterator<Item = &u8> {
        (0..9).map(move |i| &self.raster[i][column])
    }

    pub fn iter_inner_box(&self, row: usize, column: usize) -> impl Iterator<Item = &u8> {
        let row_start = (row / 3) * 3;
        let col_start = (column / 3) * 3;
        (0..3)
            .flat_map(move |dr| (0..3).map(move |dc| &self.raster[row_start + dr][col_start + dc]))
    }

    pub fn solve_all_simples(&mut self) -> (u8, u8) {
        let mut found = 0;
        let mut missing = 0;

        // Performant for loop, only iterate over the 1 values in the u16
        let mut unsolved_rows: u16 = self.unsolved_rows;
        while unsolved_rows != 0 {
            let row = unsolved_rows.trailing_zeros() as usize;
            unsolved_rows &= unsolved_rows - 1; // clear the lowest set bit

            // Performant for loop, only iterate over the 1 values in the u16
            let mut unsolved_columns: u16 = self.unsolved_columns;
            while unsolved_columns != 0 {
                let col = unsolved_columns.trailing_zeros() as usize;
                unsolved_columns &= unsolved_columns - 1; // clear the lowest set bit

                if self.raster[row][col] != 0 {
                    continue;
                }

                // Cross all options
                self.possibles_raster[row][col].options &= self.rows_options[row]
                    & self.columns_options[col]
                    & self.inner_box_options[box_index!(row, col)];

                if let Some(v) = self.possibles_raster[row][col].get_solution() {
                    self.set_value(row, col, v as u8);
                    found += 1;
                } else {
                    missing += 1;
                }
            }
        }
        (found, missing)
    }

    pub fn verify(&self, solution: &Solution) -> SolveState {
        for row in 0..9 {
            for column in 0..9 {
                let value = self.raster[row][column];
                if value == 0 {
                    return SolveState::Incomplete;
                }
                if value != solution.raster[row][column] {
                    return SolveState::Incorrect;
                }
            }
        }
        SolveState::Correct
    }
}

impl fmt::Display for Sudoku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = "+-------+-------+-------+";
        for row in 0..9 {
            if row % 3 == 0 {
                writeln!(f, "{sep}")?;
            }
            for col in 0..9 {
                if col % 3 == 0 {
                    write!(f, "| ")?;
                }
                let cell = self.raster[row][col];
                if cell == 0 {
                    write!(f, ". ")?;
                } else {
                    write!(f, "{cell} ")?;
                }
            }
            writeln!(f, "|")?;
        }
        write!(f, "{sep}")
    }
}

fn process_line(line: &str) -> Option<SolveState> {
    let (puzzle, solution) = line.split_once(',')?;
    let mut puzzle = Sudoku::from(puzzle);
    let solution = Solution::from(solution);

    loop {
        let (found, missing) = puzzle.solve_all_simples();
        if found == 0 {
            if missing == 0 {
                break;
            } else {
                // TODO:cast inner box line rays
                // TODO: if 4 variables, where 2 only contain 2 options => eliminate others
                break;
            }
        }
    }
    Some(puzzle.verify(&solution))
}

#[cfg(test)]
mod test {
    use crate::{ALL_OPTIONS, Possibles, SolveState, Sudoku, process_line};

    #[test]
    fn test_one_puzzle() {
        let puzzle_example = "070000043040009610800634900094052000358460020000800530080070091902100005007040802,\
            679518243543729618821634957794352186358461729216897534485276391962183475137945862";

        let result = process_line(puzzle_example).expect("Some result");

        assert_eq!(result, SolveState::Correct);
    }
    #[test]
    fn test_unknowns() {
        let puzzle_example =
            "679518243543729618800634900094052000358460020000800530080070091902100005007040802";
        let sudoku = Sudoku::from(puzzle_example);
        println!("{}", sudoku);
        assert_eq!(format!("{:09b}", sudoku.unsolved_rows), "111111100");
    }

    #[test]
    fn test_possibles() {
        let mut possibles = Possibles {
            options: 0b0000_1000_0000,
        };
        let result = possibles.get_solution().expect("A result");
        assert_eq!(result, 8);
        possibles = Possibles {
            options: 0b0001_1000_0000,
        };
        assert_eq!(None, possibles.get_solution());
    }

    #[test]
    fn test_bit_operations() {
        let index = 3_usize;
        let bit_mask = 1u16 << index;
        let visualized = format!("{:08b}", bit_mask);
        assert_eq!(visualized, "00001000");
        let bit_mask = ALL_OPTIONS ^ bit_mask;
        let visualized = format!("{:b}", bit_mask);
        assert_eq!(visualized, "111110111");
    }

    #[test]
    fn test_trailing_zeros() {
        let value: u16 = 0b0000_0100;
        println!("{} = {}", value, value.trailing_zeros());
    }
}
