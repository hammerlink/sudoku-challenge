use std::fmt;
use std::slice::Iter;

pub const ALL_OPTIONS: u16 = 511;

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

macro_rules! execute_algorithm {
    ($self:expr, $found:expr) => {
        if $self.unsolved_rows == 0 {
            break;
        }
        if $found {
            continue;
        }
    };
}

/// Iterates over each set bit in a u16, producing the bit index.
/// `b0001_1000_0000` => iterates [7, 8]
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

/// Like for_each_bit but yields sudoku digit values (1-based).
/// `0b1_0001_0010` => iterates [2, 5, 9]
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

#[derive(PartialEq, Debug)]
pub enum SolveState {
    Correct,
    Incorrect,
    Incomplete,
}

#[derive(Copy, Clone)]
pub struct Possibles {
    pub options: u16,
}

impl Possibles {
    pub fn default() -> Self {
        Self { options: ALL_OPTIONS }
    }

    pub fn from(value: u8) -> Self {
        if value == 0 {
            return Self { options: ALL_OPTIONS };
        }
        Self { options: 1 << (value - 1) }
    }

    pub fn get_solution(&self) -> Option<usize> {
        if self.options.count_ones() != 1 {
            return None;
        }
        Some(self.options.trailing_zeros() as usize + 1)
    }
}

pub struct Solution {
    pub raster: [[u8; 9]; 9],
}

impl Solution {
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
}

#[derive(Clone)]
pub struct Sudoku {
    pub raster: [[u8; 9]; 9],
    /// Bitmask: which rows still have unsolved cells
    pub unsolved_rows: u16,
    /// Bitmask: which columns still have unsolved cells
    pub unsolved_columns: u16,
    /// Bitmask: which 3×3 boxes still have unsolved cells
    pub unsolved_inner_boxes: u16,
    /// Remaining candidate digits for each row
    pub rows_options: [u16; 9],
    /// Remaining candidate digits for each column
    pub columns_options: [u16; 9],
    pub inner_box_options: [u16; 9],
    pub possibles_raster: [[Possibles; 9]; 9],
}

#[allow(unused)]
impl Sudoku {
    /// Applies algorithms cheapest-first; restarts from the top whenever progress is made.
    pub fn solve(&mut self) {
        loop {
            execute_algorithm!(self, self.solve_all_simples());
            execute_algorithm!(self, self.solve_exclusive_options());
            execute_algorithm!(self, self.exclude_by_ray_casting());
            execute_algorithm!(self, self.exclude_by_testing());
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

                if val > 0 {
                    possibles_raster[row][col] = Possibles::from(val);
                    rows_options[row] &= ALL_OPTIONS ^ 1 << (val - 1);
                    columns_options[col] &= ALL_OPTIONS ^ 1 << (val - 1);
                    inner_box_options[box_index!(row, col)] &= ALL_OPTIONS ^ 1 << (val - 1);
                }
            }
        }
        let unsolved_rows: u16 = rows_options.iter().enumerate().fold(0, |mut r, (i, v)| {
            if *v > 0 { r |= 1u16 << i; }
            r
        });
        let unsolved_columns: u16 = columns_options.iter().enumerate().fold(0, |mut r, (i, v)| {
            if *v > 0 { r |= 1u16 << i; }
            r
        });
        let unsolved_inner_boxes: u16 = inner_box_options.iter().enumerate().fold(0, |mut r, (i, v)| {
            if *v > 0 { r |= 1u16 << i; }
            r
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
        (0..3).flat_map(move |dr| {
            (0..3).map(move |dc| &self.raster[row_start + dr][col_start + dc])
        })
    }

    /// Intersects row/col/box candidate sets for each empty cell; fills any cell with exactly one option.
    pub fn solve_all_simples(&mut self) -> bool {
        let mut found = false;

        for_each_bit!(self.unsolved_rows, row, {
            for_each_bit!(self.unsolved_columns, col, {
                if self.raster[row][col] != 0 {
                    continue;
                }

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

    /// If a value fits only one cell in its row, column, or box, it is forced there.
    pub fn solve_exclusive_options(&mut self) -> bool {
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

    /// If a candidate is confined to one row/col within a box, eliminates it from the rest of that row/col (pointing pairs/triples).
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
                    if row_col >= col && row_col <= col + 2 {
                        continue;
                    }
                    let opts = options!(self, row + i, row_col);
                    if opts & (ALL_OPTIONS ^ exclusive_row) != opts {
                        self.possibles_raster[row + i][row_col].options &= ALL_OPTIONS ^ exclusive_row;
                        options_excluded = true;
                    }
                }
            }

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
                    if col_row >= row && col_row <= row + 2 {
                        continue;
                    }
                    let opts = options!(self, col_row, col + i);
                    if opts & (ALL_OPTIONS ^ exclusive_col) != opts {
                        self.possibles_raster[col_row][col + i].options &= ALL_OPTIONS ^ exclusive_col;
                        options_excluded = true;
                    }
                }
            }
        });
        options_excluded
    }

    /// Tries the lowest candidate in the first unsolved cell on a clone; accepts it if the clone
    /// solves fully, otherwise eliminates it.
    pub fn exclude_by_testing(&mut self) -> bool {
        let row = self.unsolved_rows.trailing_zeros() as usize;
        for_each_bit!(self.unsolved_columns, col, {
            let opts = options!(self, row, col);
            if opts.count_ones() <= 1 {
                continue;
            }
            let value = opts.trailing_zeros() as u8 + 1;
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
pub struct PossiblesView<'a>(pub &'a Sudoku);

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
