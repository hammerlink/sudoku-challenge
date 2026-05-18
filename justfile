tdoku_build := "tdoku/build"
tdoku_data  := "tdoku/data"
lib         := "target/release/libsudoku_solver.so"

# build the Rust library and the tdoku benchmark binary
build:
    git submodule update --init
    @if [ ! -d "{{tdoku_data}}" ]; then unzip tdoku/data.zip -d tdoku; fi
    cargo build --release
    cd tdoku && ./BUILD.sh run_benchmark -DHAMMERLINK_SUDOKU=on

# run the full benchmark suite against all datasets
bench: build
    {{tdoku_build}}/run_benchmark -s hammerlink,tdoku \
        {{tdoku_data}}/puzzles0_kaggle \
        {{tdoku_data}}/puzzles1_unbiased \
        {{tdoku_data}}/puzzles2_17_clue \
        {{tdoku_data}}/puzzles3_magictour_top1465 \
        {{tdoku_data}}/puzzles4_forum_hardest_1905 \
        {{tdoku_data}}/puzzles5_forum_hardest_1905_11+ \
        {{tdoku_data}}/puzzles6_forum_hardest_1106 \
        {{tdoku_data}}/puzzles7_serg_benchmark \
        {{tdoku_data}}/puzzles8_gen_puzzles

# quick smoke-test bench (1 000 puzzles, short warmup)
bench-quick: build
    {{tdoku_build}}/run_benchmark -t3 -w1 -n1000 -s hammerlink,tdoku \
        {{tdoku_data}}/puzzles0_kaggle \
        {{tdoku_data}}/puzzles1_unbiased \
        {{tdoku_data}}/puzzles3_magictour_top1465

# run Rust tests
test:
    cargo test
