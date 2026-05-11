use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::slice::Iter;

use rayon::prelude::*;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB read buffer

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

enum SolveState {
    Correct,
    Incorrect,
    Incomplete,
}

#[derive(Copy, Clone)]
struct Possibles {
    options: [bool; 9],
    count: u8,
}
impl Possibles {
    fn default() -> Self {
        Self {
            options: [true; 9],
            count: 9,
        }
    }

    fn get_solution(&self) -> Option<usize> {
        if self.count != 1 {
            return None;
        }
        self.options.iter().enumerate().find_map(|(i, v)| {
            if *v {
                return Some(i + 1);
            }
            None
        })
    }

    fn set_value(&mut self, value: u8) {
        if value == 0 {
            return;
        }
        let index = value as usize - 1;
        if self.options[index] {
            self.options[index] = false;
            self.count -= 1;
        }
    }
}

// example: 070000043040009610800634900094052000358460020000800530080070091902100005007040802,679518243543729618821634957794352186358461729216897534485276391962183475137945862
struct Sudoku {
    pub raster: [[u8; 9]; 9],
    pub rows_count: [u8; 9],
    pub columns_count: [u8; 9],
    pub possibles_raster: [[Possibles; 9]; 9],
}

#[allow(unused)]
impl Sudoku {
    pub fn from(input: &str) -> Self {
        let bytes = input.as_bytes();
        let mut raster = [[0u8; 9]; 9];
        let mut rows_count: [u8; 9] = [0u8; 9];
        let mut columns_count: [u8; 9] = [0u8; 9];
        let possibles_raster: [[Possibles; 9]; 9] = [[Possibles::default(); 9]; 9];
        for row in 0..9 {
            for col in 0..9 {
                let val = bytes[row * 9 + col] - b'0';
                raster[row][col] = val;
                if val > 0 {
                    rows_count[row] += 1;
                    columns_count[col] += 1;
                }
            }
        }
        Self {
            raster,
            rows_count,
            columns_count,
            possibles_raster,
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

    pub fn solve_all_simples(&mut self) -> u8 {
        let mut found = 0;
        for row in 0..9 {
            for col in 0..9 {
                if self.raster[row][col] != 0 {
                    continue;
                }

                let determined = {
                    let (raster, possibles) = (&self.raster, &mut self.possibles_raster);
                    let p = &mut possibles[row][col];

                    // Iter rows
                    for &v in &raster[row] {
                        p.set_value(v);
                    }
                    // Iter columns
                    (0..9).for_each(|r| {
                        p.set_value(raster[r][col]);
                    });
                    // Iter inner boxes
                    let (rs, cs) = ((row / 3) * 3, (col / 3) * 3);
                    for dr in 0..3 {
                        for dc in 0..3 {
                            p.set_value(raster[rs + dr][cs + dc]);
                        }
                    }
                    p.get_solution()
                };

                if let Some(v) = determined {
                    self.raster[row][col] = v as u8;
                    found += 1;
                }
            }
        }
        found
    }

    pub fn verify(&self, solution: &Sudoku) -> SolveState {
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
    let solution = Sudoku::from(solution);

    while puzzle.solve_all_simples() > 0 {}
    Some(puzzle.verify(&solution))
}

#[cfg(test)]
mod test {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use crate::{BUFFER_SIZE, process_line};

    #[test]
    fn test_one_puzzle() {
        let file = File::open("sudoku.csv").unwrap();
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);

        let mut line = String::with_capacity(256);
        let mut first = true;

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).unwrap();
            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim_end_matches(['\n', '\r']);

            // Skip header row
            if first {
                first = false;
                if trimmed.starts_with("puzzle") {
                    continue;
                }
            }

            println!("{trimmed}");
            process_line(trimmed);
            break;
        }
    }
}
