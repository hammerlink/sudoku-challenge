# Sudoku Solver

A personal experiment in Rust: solve 9 million sudoku puzzles the way a human would.

## Background

A colleague mentioned he had done this challenge a long time ago. That was enough. I downloaded
a dataset from [Hugging Face](https://huggingface.co) containing 9 million sudoku puzzles, each
paired with its solution, and set out to replicate the human solving process in code — no brute
force search, just the logical techniques you would apply yourself with a pen and paper.

Along the way this became a hands-on training ground for Rust macros and bit manipulation.

## Dataset

- **Source:** Hugging Face — 9 million sudoku puzzle/solution pairs in CSV format
- **Format:** `puzzle,solution` — 81-character strings, `0` for empty cells

## How It Solves

Algorithms are applied in order from cheapest to most expensive. When any algorithm makes
progress the loop restarts from the top.

### 1. Simple elimination (`solve_all_simples`)

For each empty cell, intersect the remaining options of its row, column, and 3×3 box.
If only one candidate survives, fill it in. This is the first thing any human tries.

### 2. Exclusive options (`solve_exclusive_options`)

Scan each row, column, and inner box for values that can only land in a single cell.
Even when a cell has multiple candidates, a value that fits nowhere else in its group
is forced there.

### 3. Ray casting (`exclude_by_ray_casting`)

Look inside each unsolved 3×3 box: if a candidate value is confined to a single row
(or column) within the box, it cannot appear in that same row (or column) outside the
box — eliminate it. This mirrors the human technique of "pointing pairs/triples".

### 4. Testing a value (`exclude_by_testing`)

Last resort: pick the first unsolved cell with multiple candidates, try the lowest one,
and run the full solve on a clone. If it leads to a complete solution, accept it.
If not, eliminate that candidate and continue. This brought the completion rate from
~98% to 100%.

## Progression

The commit history tells the story of how the solution evolved:

| Step | What changed | Result |
|------|-------------|--------|
| Initial attempt | Basic constraint propagation | 80% solved, ~61 sec |
| Rayon | Parallel iteration over all 9M puzzles | Big speedup |
| Byte operations | Switch from char parsing to byte ops | ~5.9 sec |
| Bitmask crossing | Represent candidates as `u16` bitmasks | ~2.4 sec, 2× faster |
| Track unsolved sets | Bitmask of unsolved rows/cols/boxes, skip solved | Fewer iterations |
| Exclusive options | Detect forced values across row/col/box | 97.72% solved |
| Macros (`for_each_bit`, `for_each_bit_value`) | Iterate set bits without boilerplate | Cleaner code |
| `memmap2` | Memory-mapped file reading | Faster I/O |
| Ray casting | Eliminate candidates across box boundaries | ~98% solved |
| Testing a value | Try & verify a candidate on a clone | 100% solved, ~2.95 sec |

## Final Result

```
Processed  9 000 000 puzzles
Completed  9 000 000   100%
Incorrect          0     0%
```

Time: ~2.95 seconds on all 9 million puzzles.

## Key Techniques Learned

**Bitmask representation** — each cell's candidate set is a `u16` where bit `n` means
digit `n+1` is still possible. Intersection, elimination, and single-candidate detection
all reduce to bitwise AND/XOR and `count_ones()` / `trailing_zeros()`.

**Rust macros** — repetitive bit-iteration patterns were factored into `for_each_bit!`
and `for_each_bit_value!` macros, and compound expressions like `exclusive_options!`
keep the algorithm code readable.

**Rayon** — a single `.into_par_iter()` parallelises the 9M puzzle workload across all
cores with no manual thread management.

## Running

```bash
# Place sudoku.csv in the project root, then:
cargo run --release
```
