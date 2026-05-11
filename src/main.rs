use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::slice::Iter;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB read buffer

fn main() -> io::Result<()> {
    let file = File::open("sudoku.csv")?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let mut line = String::with_capacity(256);
    let mut count = 0u64;
    let mut first = true;

    let mut incomplete = 0u64;
    let mut incorrect = 0u64;
    let mut complete = 0u64;

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
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

        if let Some(result) = process_line(trimmed) {
            match result {
                SolveState::Correct => complete += 1,
                SolveState::Incorrect => incorrect += 1,
                SolveState::Incomplete => incomplete += 1,
            }
        }
        count += 1;
    }

    eprintln!("Processed {count} puzzles");
    let completed_percent = complete as f64 / count as f64 * 100.0f64;
    println!("Completed {complete} {completed_percent}");
    let incompleted_percent = incomplete as f64 / count as f64 * 100.0f64;
    println!("Incompleted {incomplete} {incompleted_percent}");
    Ok(())
}

enum SolveState {
    Correct,
    Incorrect,
    Incomplete,
}

// example: 070000043040009610800634900094052000358460020000800530080070091902100005007040802,679518243543729618821634957794352186358461729216897534485276391962183475137945862
struct Sudoku {
    pub raster: [[u8; 9]; 9],
}

#[allow(unused)]
impl Sudoku {
    pub fn from(input: &str) -> Self {
        let bytes = input.as_bytes();
        let mut raster = [[0u8; 9]; 9];
        for row in 0..9 {
            for col in 0..9 {
                raster[row][col] = bytes[row * 9 + col] - b'0';
            }
        }
        Self { raster }
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
        // Iterate all rows, cross with columns & boxes
        let mut possible_count: u8 = 9;
        // Index equals the number
        let mut possibles: [bool; 9] = [false; 9];
        for row in 0..9 {
            for col in 0..9 {
                let value = self.raster[row][col];
                if value != 0 {
                    continue;
                }
                possible_count = 9;
                possibles = [true; 9];

                self.iter_row(row).for_each(|v| {
                    if (*v > 0 && possibles[unsafe { *v as usize - 1 }]) {
                        possible_count -= 1;
                        possibles[unsafe { *v as usize - 1 }] = false;
                    }
                });
                self.iter_column(col).for_each(|v| {
                    if (*v > 0 && possibles[unsafe { *v as usize - 1 }]) {
                        possible_count -= 1;
                        possibles[unsafe { *v as usize - 1 }] = false;
                    }
                });
                self.iter_inner_box(row, col).for_each(|v| {
                    if (*v > 0 && possibles[unsafe { *v as usize - 1 }]) {
                        possible_count -= 1;
                        possibles[unsafe { *v as usize - 1 }] = false;
                    }
                });
                if possible_count == 1 {
                    let determined_value = possibles
                        .iter()
                        .enumerate()
                        .find_map(|(i, v)| {
                            if (*v) {
                                return Some(i + 1);
                            }
                            None
                        })
                        .unwrap();
                    self.raster[row][col] = determined_value as u8;
                    // println!("Filling {row} - {col} value: {determined_value}");
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
