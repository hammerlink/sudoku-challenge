use std::fmt;
use std::fs::File;
use std::io;
use std::slice::Iter;

use memmap2::Mmap;
use rayon::prelude::*;

const ALL_OPTIONS: u16 = 511;

macro_rules! box_index {
    ($row:expr, $col:expr) => {
        ($row / 3) * 3 + ($col / 3)
    };
}

macro_rules! options {
    ($puzzle:expr, $row:expr, $col:expr) => {
        $puzzle.possibles_raster[$row][$col].options
    };
}

macro_rules! inner_box_row_options {
    ($puzzle:expr, $inner_box_row:expr, $inner_box_col:expr, $inner_box_row_index: expr) => {
        (options!(
            $puzzle,
            $inner_box_row + $inner_box_row_index,
            $inner_box_col
        ) | options!(
            $puzzle,
            $inner_box_row + $inner_box_row_index,
            $inner_box_col + 1
        ) | options!(
            $puzzle,
            $inner_box_row + $inner_box_row_index,
            $inner_box_col + 2
        ))
    };
}

macro_rules! inner_box_col_options {
    ($puzzle:expr, $inner_box_row:expr, $inner_box_col:expr, $inner_box_col_index: expr) => {
        (options!(
            $puzzle,
            $inner_box_row,
            $inner_box_col + $inner_box_col_index
        ) | options!(
            $puzzle,
            $inner_box_row + 1,
            $inner_box_col + $inner_box_col_index
        ) | options!(
            $puzzle,
            $inner_box_row + 2,
            $inner_box_col + $inner_box_col_index
        ))
    };
}

macro_rules! exclusive_options {
    ($base:expr, $other1:expr, $other2:expr) => {
        (($base ^ $other1) & $base) & (($base ^ $other2) & $base)
    };
}

fn main() -> io::Result<()> {
    let file = File::open("sudoku.csv")?;
    let mmap = unsafe { Mmap::map(&file)? };
    let content = std::str::from_utf8(&mmap).expect("valid UTF-8");

    let lines: Vec<&str> = content
        .lines()
        .skip_while(|l| l.starts_with("puzzle"))
        .collect();

    let (complete, incomplete, incorrect) = lines
        .into_par_iter()
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

#[derive(Clone)]
struct Sudoku {
    pub raster: [[u8; 9]; 9],
    /// Which row indeces are not completely filled yet
    /// 0_0000_1001 => 4th and 1st row contain missing values
    pub unsolved_rows: u16,
    /// Which columns have unknowns, similar to unsolved_rows
    pub unsolved_columns: u16,
    /// Which inner boxes have unknowns, similar to unsolved_rows
    pub unsolved_inner_boxes: u16,
    /// What options are still open for each row
    /// index-0: 0_0001_0010 => first row has 2 and 5 missing
    pub rows_options: [u16; 9],
    /// What options are still open for each column
    /// index-0: 0_0001_0010 => first column has 2 and 5 missing
    pub columns_options: [u16; 9],
    pub inner_box_options: [u16; 9],
    pub possibles_raster: [[Possibles; 9]; 9],
}

macro_rules! execute_algorithm {
    ($self:expr, $found:expr) => {
        // The sudoku is solved, terminate the solve algorithm
        if $self.unsolved_rows == 0 {
            break;
        }
        // Algorithms are sorted by execution cost, try a simpler algorithm first
        if $found {
            continue;
        }
    };
}

/// Iterates over each 1 value in a u16 (or other value that supports trailing_zeros)
/// Produces the index of the 1 each time
/// Example u16 b0001_1000_0000 => iterates \[8,9\]
macro_rules! for_each_bit {
    ($bits:expr, $var:ident, $body:block) => {
        let mut _bits = $bits;
        while _bits != 0 {
            let $var = _bits.trailing_zeros() as usize;
            _bits &= _bits - 1;
            $body
        }
    };
}

/// Identical to for_each_bit but convert it to a sudoku value in u8
/// 1_0001_0010, iterates over \[2,5,9\]
macro_rules! for_each_bit_value {
    ($bits:expr, $var:ident, $body:block) => {
        let mut _bits = $bits;
        while _bits != 0 {
            let $var = _bits.trailing_zeros() as u8 + 1;
            _bits &= _bits - 1;
            $body
        }
    };
}

#[allow(unused)]
impl Sudoku {
    pub fn solve(&mut self) {
        loop {
            execute_algorithm!(self, self.solve_all_simples());
            execute_algorithm!(self, self.solve_exclusive_options());
            execute_algorithm!(self, self.exclude_by_ray_casting());

            execute_algorithm!(self, self.exclude_by_testing());

            // No results, stop
            break;
        }
    }
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
        let unsolved_inner_boxes: u16 =
            inner_box_options
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
            unsolved_inner_boxes,
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
        if self.inner_box_options[box_index!(row, col)] == 0 {
            self.unsolved_inner_boxes &= ALL_OPTIONS ^ 1 << (box_index!(row, col));
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

    /// Returns true if any values have been solved
    pub fn solve_all_simples(&mut self) -> bool {
        let mut found = false;

        for_each_bit!(self.unsolved_rows, row, {
            for_each_bit!(self.unsolved_columns, col, {
                if self.raster[row][col] != 0 {
                    continue;
                }

                // Cross all options
                self.possibles_raster[row][col].options &= self.rows_options[row]
                    & self.columns_options[col]
                    & self.inner_box_options[box_index!(row, col)];

                if let Some(v) = self.possibles_raster[row][col].get_solution() {
                    self.set_value(row, col, v as u8);
                    found = true;
                }
            });
        });

        found
    }

    pub fn solve_exclusive_options(&mut self) -> bool {
        // Iterate over rows
        for_each_bit!(self.unsolved_rows, row, {
            for_each_bit_value!(self.rows_options[row], value, {
                let mut options: Vec<(usize, usize)> = vec![];
                for col in 0..9 {
                    if self.possibles_raster[row][col].options & 1 << (value - 1) != 0 {
                        options.push((row, col));
                    }
                }
                if options.len() == 1 {
                    self.set_value(options[0].0, options[0].1, value);
                    return true;
                }
            });
        });

        // Iterate over columns
        for_each_bit!(self.unsolved_columns, col, {
            for_each_bit_value!(self.columns_options[col], value, {
                let mut options: Vec<(usize, usize)> = vec![];
                for row in 0..9 {
                    if self.possibles_raster[row][col].options & 1 << (value - 1) != 0 {
                        options.push((row, col));
                    }
                }
                if options.len() == 1 {
                    self.set_value(options[0].0, options[0].1, value);
                    return true;
                }
            });
        });

        // Iterate over all inner box options
        for_each_bit!(self.unsolved_inner_boxes, inner_box_index, {
            for_each_bit_value!(self.inner_box_options[inner_box_index], value, {
                let mut options: Vec<(usize, usize)> = vec![];
                let row_start = (inner_box_index / 3) * 3;
                let col_start = (inner_box_index % 3) * 3;
                for dr in 0..3 {
                    for dc in 0..3 {
                        let row = row_start + dr;
                        let col = col_start + dc;
                        if self.possibles_raster[row][col].options & 1 << (value - 1) != 0 {
                            options.push((row, col));
                        }
                    }
                }
                if options.len() == 1 {
                    self.set_value(options[0].0, options[0].1, value);
                    return true;
                }
            });
        });
        false
    }

    pub fn exclude_by_ray_casting(&mut self) -> bool {
        let mut options_excluded = false;
        for_each_bit!(self.unsolved_inner_boxes, inner_box_index, {
            let row = (inner_box_index / 3) * 3;
            let col = (inner_box_index % 3) * 3;

            let inner_box_rows: [u16; 3] = [
                inner_box_row_options!(self, row, col, 0) & self.inner_box_options[inner_box_index],
                inner_box_row_options!(self, row, col, 1) & self.inner_box_options[inner_box_index],
                inner_box_row_options!(self, row, col, 2) & self.inner_box_options[inner_box_index],
            ];
            let inner_box_cols: [u16; 3] = [
                inner_box_col_options!(self, row, col, 0) & self.inner_box_options[inner_box_index],
                inner_box_col_options!(self, row, col, 1) & self.inner_box_options[inner_box_index],
                inner_box_col_options!(self, row, col, 2) & self.inner_box_options[inner_box_index],
            ];

            // Row options
            // Search for exclusives, all options that are in no other row
            for i in 0..3 {
                let exclusive_row = exclusive_options!(
                    inner_box_rows[i],
                    inner_box_rows[(i + 1) % 3],
                    inner_box_rows[(i + 2) % 3]
                );
                if exclusive_row == 0 {
                    continue;
                }
                for row_col in 0..9 {
                    // Skip within inner box
                    if row_col >= col && row_col <= col + 2 {
                        continue;
                    }

                    let options = options!(self, row + i, row_col);
                    if options & (ALL_OPTIONS ^ exclusive_row) != options {
                        self.possibles_raster[row + i][row_col].options &=
                            ALL_OPTIONS ^ exclusive_row;
                        options_excluded = true;
                    }
                }
            }

            // Search for exclusives, all options that are in no other col
            for i in 0..3 {
                let exclusive_col = exclusive_options!(
                    inner_box_cols[i],
                    inner_box_cols[(i + 1) % 3],
                    inner_box_cols[(i + 2) % 3]
                );
                if exclusive_col == 0 {
                    continue;
                }
                for col_row in 0..9 {
                    // Skip within inner box
                    if col_row >= row && col_row <= row + 2 {
                        continue;
                    }
                    let options = options!(self, col_row, col + i);
                    if options & (ALL_OPTIONS ^ exclusive_col) != options {
                        self.possibles_raster[col_row][col + i].options &=
                            ALL_OPTIONS ^ exclusive_col;
                        options_excluded = true;
                    }
                }
            }
        });
        options_excluded
    }

    pub fn exclude_by_testing(&mut self) -> bool {
        // Test out first encounter
        let row = self.unsolved_rows.trailing_zeros() as usize;
        for_each_bit!(self.unsolved_columns, col, {
            let options = options!(self, row, col);
            if options.count_ones() <= 1 {
                continue;
            }
            let value = options.trailing_zeros() as u8 + 1;
            let mut copy = self.clone();

            copy.set_value(row, col, value);
            copy.solve();
            if copy.unsolved_rows == 0 {
                self.set_value(row, col, value);
            } else {
                self.possibles_raster[row][col].options &= ALL_OPTIONS ^ 1 << (value - 1);
            }
            return true;
        });
        false
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

#[allow(unused)]
struct PossiblesView<'a>(&'a Sudoku);

impl<'a> fmt::Display for PossiblesView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = "+-------------------------------+-------------------------------+-------------------------------+";
        for row in 0..9 {
            if row % 3 == 0 {
                writeln!(f, "{sep}")?;
            }
            for col in 0..9 {
                if col % 3 == 0 {
                    write!(f, "| ")?;
                }
                let options = self.0.possibles_raster[row][col].options;
                if options.count_ones() == 1 {
                    write!(f, "--------- ")?;
                } else {
                    write!(f, "{:09b} ", options)?;
                }
            }
            writeln!(f, "|")?;
        }
        write!(f, "{sep}")
    }
}

#[allow(unused)]
impl Sudoku {
    pub fn possibles_view(&self) -> PossiblesView<'_> {
        PossiblesView(self)
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

static mut TRIGGERED: bool = false;
fn process_line(line: &str) -> Option<SolveState> {
    let (puzzle, solution) = line.split_once(',')?;
    let mut puzzle = Sudoku::from(puzzle);
    let solution = Solution::from(solution);

    puzzle.solve();

    let result = puzzle.verify(&solution);
    // if unsafe { !TRIGGERED } && result == SolveState::Incomplete {
    //     println!("{line}");
    //     unsafe {
    //         TRIGGERED = true;
    //     }
    // }
    Some(result)
}

#[cfg(test)]
mod test {
    use crate::{ALL_OPTIONS, Possibles, Solution, SolveState, Sudoku, process_line};

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

    #[test]
    fn incomplete_result() {
        // Failing test before solve_exclusive_options
        let puzzle = "490703000100500730300900084070000103036000005541600900004009000050208410610000002\
            ,495783621182546739367921584279854163836197245541632978724319856953268417618475392";
        let (puzzle, solution) = puzzle.split_once(',').expect("parsed");
        let mut puzzle = Sudoku::from(puzzle);
        println!("{}", puzzle);
        let solution = Solution::from(solution);

        puzzle.solve();
        println!("{}", puzzle);

        let result = puzzle.verify(&solution);
        assert_eq!(result, SolveState::Correct);
    }

    #[test]
    fn incomplete_result_2() {
        let puzzle = "032000071504000008081040056300870020000004100000350600000008010000915860000000390,\
            632589471574126938981743256346871529257694183198352647469238715723915864815467392";
        let (puzzle, solution) = puzzle.split_once(',').expect("parsed");
        let mut puzzle = Sudoku::from(puzzle);
        println!("{}", puzzle);
        let solution = Solution::from(solution);

        puzzle.solve();
        // println!("{}", puzzle.possibles_view());
        println!("{}", puzzle);

        let result = puzzle.verify(&solution);

        assert_eq!(result, SolveState::Correct);
    }
}
