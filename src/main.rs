use std::fs::File;
use std::io::{self, BufRead, BufReader};

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MB read buffer

fn main() -> io::Result<()> {
    let file = File::open("sudoku.csv")?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);

    let mut line = String::with_capacity(256);
    let mut count = 0u64;
    let mut first = true;

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

        process_line(trimmed);
        count += 1;
    }

    eprintln!("Processed {count} puzzles");
    Ok(())
}

fn process_line(line: &str) {
    let Some((puzzle, solution)) = line.split_once(',') else {
        return;
    };
    // TODO: solve / validate puzzle here
    let _ = (puzzle, solution);
}
