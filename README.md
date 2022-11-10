# Minesweeper — the classic game in Rust, compiled to WebAssembly

[![ci](https://github.com/sushruth31/minesweeper-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/sushruth31/minesweeper-rust/actions/workflows/ci.yml)

A playable minesweeper board rendered with [Yew](https://yew.rs) and served as a
single wasm module by [Trunk](https://trunkrs.dev). Written over a couple of
evenings to learn Yew's hook API, but the interesting part is not the UI: it is
keeping the rules — first-click safety, flood fill, win detection — in a module
that knows nothing about the browser, so they can be tested with a plain
`cargo test` instead of a headless Chrome.

## Stack

- **Rust 2021 / wasm32-unknown-unknown** — the whole game, logic and view.
- **Yew 0.19** — function components and `use_reducer`, which maps cleanly onto
  a pure `(state, action) -> state` transition.
- **Trunk** — asset pipeline and dev server; runs `wasm-bindgen` and fingerprints
  the CSS so nothing needs a hand-written build script.
- **rand** — `getrandom`'s `js` feature backs it with `crypto.getRandomValues`
  in the browser. Every rules function that needs randomness takes the `Rng` as
  a parameter rather than reaching for a thread-local, so the tests hand it a
  seeded `StdRng` and get byte-identical boards.

No runtime dependencies beyond those, and no JavaScript of my own.

## Running it

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

trunk serve --open          # http://localhost:8080, rebuilds on save
```

Board size and mine count are build-time settings (a wasm module has no process
environment). Copy `.env.example`, edit it, and export before building:

```bash
cp .env.example .env
set -a; . ./.env; set +a
trunk build --release       # output in dist/
```

Everything is optional: unset variables fall back to a 10x10 grid with 15 mines.
A malformed or impossible value aborts start-up with an on-page error naming the
variable rather than quietly clamping.

## Architecture

```
index.html ──> src/main.rs ──> app::App ──> app::Game
                    │                          │
                    │  Config::from_build_env  │  dispatch(Action)
                    ▼                          ▼
             src/config.rs             GameState::apply  (src/game.rs)
                                               │
                                               ▼
                                        Board  (rules, no DOM)
```

| Module          | Responsibility                                                              | Builds for |
|-----------------|-----------------------------------------------------------------------------|------------|
| `src/game.rs`   | `Board`, flood fill, mine placement, win/loss. Zero framework imports.       | host + wasm |
| `src/config.rs` | Board dimensions from the build environment, validated once.                 | host + wasm |
| `src/app.rs`    | Yew components. Renders `Board`, emits `Action`, holds no rules.              | wasm only  |
| `src/main.rs`   | Mounts the app, or prints a hint if you `cargo run` it on the host.           | both       |

`app` is behind `#[cfg(target_arch = "wasm32")]`, and Yew is declared under
`[target.'cfg(target_arch = "wasm32")'.dependencies]`, so a host `cargo test`
never compiles a line of DOM code.

## Design notes

- **The first click is never a mine.** Mines are not placed at construction
  time; the board stays empty until the first `reveal` *that actually uncovers
  something*, which then lays them at random over every cell **except** the
  clicked one and its eight neighbours. That guarantees the opening move always
  cracks open a blank region instead of ending the game on move one — the single
  most common bug in a naive implementation. The ordering inside `reveal` is
  load-bearing: seeding above the flagged/already-uncovered guard would let a
  click on a flagged cell consume the guarantee and hand the player an
  unprotected first real move, so the guard runs first and a test pins it down.
  It also has a consequence worth stating: nine cells can never
  hold a mine, so `Config` rejects any mine count above `width * height - 9` at
  start-up rather than looping forever looking for a free square.
- **Generation and flood fill are both O(n) in the number of cells.** Mines are
  drawn with a partial Fisher–Yates (`choose_multiple`) over the candidate
  indices — one pass, no rejection sampling. Adjacency counts are a second pass
  computed into a scratch vector and then written back, which keeps the borrow
  checker happy without a `RefCell`. The reveal is an iterative flood fill over
  an explicit `Vec` stack: recursion would be O(depth) on a wasm stack that
  can't grow, and a blank region can span the whole board. Each cell is
  uncovered once and pushes at most eight neighbours, so the fill is O(8n) pops.
- **"Still playing" is not a variant.** `Board::reveal` returns
  `Option<GameResult>`, not a three-way `Outcome::{Continue, Won, Lost}`. The
  three-way version lets a `Continue` leak into the terminal state where nothing
  sensible can be rendered for it; with `Option`, the state machine can only
  hold `Won` or `Lost` and every match on it is total.
- **The view cannot cheat.** `GameState::apply(&self, action, rng) -> Self` is a
  pure transition; the Yew layer only wraps it in `Reducible` and turns the
  resulting `Board` into `<div>`s. That is why the 28 tests below can drive the
  whole game — including "the game is over, ignore this click" — without
  mounting a component.
- **Release profile, measured.** `opt-level = "s"` + fat LTO +
  `codegen-units = 1` + `panic = "abort"` takes the shipped wasm from 279 KiB to
  193 KiB — a 31% cut, measured by deleting `[profile.release]` and comparing
  `dist/*.wasm`. For a page whose only payload *is* the binary that is the whole
  performance budget, and the price is paid entirely in the release link step:
  `codegen-units = 1` serialises it, while `trunk serve` runs the dev profile
  and is untouched.

## Tests

```bash
cargo test                                       # 28 tests, host toolchain, no browser
cargo clippy --all-targets -- -D warnings
trunk build --release                            # the wasm bundle
```

The suite targets the rules a naive implementation gets wrong, and the test
names state the case:

- the opening click and all eight neighbours are mine-free, checked across 200
  seeds on a board at maximum legal mine density;
- a click that reveals nothing — a flagged cell — does not spend that
  guarantee: the mines are laid by the first click that actually opens a cell,
  not by the first click that arrives;
- flood fill reveals the numbered border of a blank region but never steps past
  it, and never uncovers a mine;
- flood fill skips flagged cells, so a marked guess survives the sweep;
- adjacency counts do not wrap around the row boundary — the classic row-major
  indexing bug where `(0, width-1)` and `(1, 0)` look adjacent;
- neighbour counts drop to 5 on an edge and 3 in a corner;
- a win requires every non-mine cell uncovered and is indifferent to flags;
- flags, out-of-bounds coordinates and post-game-over clicks are all no-ops;
- mine placement is reproducible for a fixed seed.
