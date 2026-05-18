pub mod solver;
pub use solver::*;

use std::slice;

/// tdoku SolverFn interface:
///   input:      81-char sudoku string ('1'-'9' for clues, '.' for empty)
///   limit:      max solutions to find (we find at most 1)
///   _flags:     solver-specific config (unused)
///   solution:   output buffer for the solved grid ('1'-'9')
///   num_guesses: out-param for backtrack count (we always write 0)
///
/// Returns 1 if solved, 0 if not.
/// features = 3 → returns_solution | returns_count
#[unsafe(no_mangle)]
pub extern "C" fn OtherSolverHammerlink(
    input: *const u8,
    _limit: usize,
    _flags: u32,
    solution: *mut u8,
    num_guesses: *mut usize,
) -> usize {
    unsafe { *num_guesses = 0 };

    // Convert '.' (empty) to '0' so Sudoku::from() can parse it.
    let input_slice = unsafe { slice::from_raw_parts(input, 81) };
    let mut buf = [b'0'; 81];
    for (i, &b) in input_slice.iter().enumerate() {
        if b != b'.' {
            buf[i] = b;
        }
    }

    let puzzle_str = unsafe { std::str::from_utf8_unchecked(&buf) };
    let mut puzzle = Sudoku::from(puzzle_str);
    puzzle.solve();

    if puzzle.unsolved_rows != 0 {
        return 0;
    }

    let solution_slice = unsafe { slice::from_raw_parts_mut(solution, 81) };
    for row in 0..9 {
        for col in 0..9 {
            solution_slice[row * 9 + col] = b'0' + puzzle.raster[row][col];
        }
    }

    1
}
