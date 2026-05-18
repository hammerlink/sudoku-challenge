use std::fs::File;
use std::io;

use memmap2::Mmap;
use rayon::prelude::*;
use sudoku_solver::{Solution, SolveState, Sudoku};

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

fn process_line(line: &str) -> Option<SolveState> {
    let (puzzle, solution) = line.split_once(',')?;
    let mut puzzle = Sudoku::from(puzzle);
    let solution = Solution::from(solution);

    puzzle.solve();
    Some(puzzle.verify(&solution))
}

#[cfg(test)]
mod test {
    use sudoku_solver::{ALL_OPTIONS, Possibles, Solution, SolveState, Sudoku};

    use super::process_line;

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
        println!("{}", puzzle);

        let result = puzzle.verify(&solution);

        assert_eq!(result, SolveState::Correct);
    }
}
