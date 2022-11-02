//! The minesweeper rules.
//!
//! Deliberately free of Yew, `web_sys` and anything else browser-shaped: this
//! module builds and runs on the host toolchain, which is what makes the rules
//! testable with a plain `cargo test`.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Mine,
    /// Number of mines touching this cell; `Adjacent(0)` is a blank cell.
    Adjacent(u8),
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Adjacent(0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellState {
    pub content: Cell,
    pub uncovered: bool,
    pub flagged: bool,
}

/// Terminal state of a game. A reveal that ends nothing returns `None`, so
/// "still playing" is not a variant anybody can forget to handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameResult {
    Won,
    Lost,
}

/// A player intent. The view layer produces these; it never mutates a board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Reveal(usize, usize),
    Flag(usize, usize),
    Restart,
}

/// Row-major grid held in a single flat allocation, indexed `row * width + col`
/// rather than as a `Vec<Vec<_>>` of independently allocated rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    config: Config,
    cells: Vec<CellState>,
    seeded: bool,
}

impl Board {
    /// An empty, unseeded board. Mines are laid on the first reveal so that the
    /// opening click can be guaranteed safe.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cells: vec![CellState::default(); config.cells()],
            seeded: false,
        }
    }

    pub fn config(&self) -> Config {
        self.config
    }

    pub fn rows(&self) -> impl Iterator<Item = &[CellState]> {
        self.cells.chunks(self.config.width)
    }

    /// Mines not yet accounted for by a flag; goes negative if the player
    /// over-flags, which is the standard behaviour.
    pub fn mines_remaining(&self) -> isize {
        let flags = self.cells.iter().filter(|cell| cell.flagged).count();
        self.config.mines as isize - flags as isize
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        row < self.config.height && col < self.config.width
    }

    /// The up-to-eight surrounding coordinates, excluding the cell itself and
    /// anything past an edge.
    pub fn neighbors(&self, row: usize, col: usize) -> impl Iterator<Item = (usize, usize)> {
        let (rows, cols) = (self.config.height, self.config.width);
        (row.saturating_sub(1)..(row + 2).min(rows))
            .flat_map(move |r| (col.saturating_sub(1)..(col + 2).min(cols)).map(move |c| (r, c)))
            .filter(move |&position| position != (row, col))
    }

    pub fn cell(&self, row: usize, col: usize) -> CellState {
        self.cells[self.index(row, col)]
    }

    fn index(&self, row: usize, col: usize) -> usize {
        row * self.config.width + col
    }

    fn is_mine(&self, row: usize, col: usize) -> bool {
        self.cell(row, col).content == Cell::Mine
    }

    /// Toggles a flag. Uncovered cells are inert, which stops a mis-click from
    /// hiding a number the player already earned.
    pub fn toggle_flag(&mut self, row: usize, col: usize) {
        if !self.contains(row, col) {
            return;
        }
        let index = self.index(row, col);
        if !self.cells[index].uncovered {
            self.cells[index].flagged = !self.cells[index].flagged;
        }
    }

    pub fn reveal<R: Rng>(&mut self, row: usize, col: usize, rng: &mut R) -> Option<GameResult> {
        if !self.contains(row, col) {
            return None;
        }
        if !self.seeded {
            self.seed((row, col), rng);
        }
        let cell = self.cell(row, col);
        if cell.flagged || cell.uncovered {
            return None;
        }
        if cell.content == Cell::Mine {
            return self.lose();
        }
        self.flood(row, col);
        self.is_cleared().then_some(GameResult::Won)
    }

    /// Lays mines uniformly at random, excluding the opening click and its
    /// neighbours. O(n) in the number of cells via a partial Fisher-Yates draw.
    fn seed<R: Rng>(&mut self, safe: (usize, usize), rng: &mut R) {
        let reserved: Vec<usize> = self
            .neighbors(safe.0, safe.1)
            .chain(std::iter::once(safe))
            .map(|(row, col)| self.index(row, col))
            .collect();
        let candidates: Vec<usize> = (0..self.cells.len())
            .filter(|index| !reserved.contains(index))
            .collect();
        for &index in candidates.choose_multiple(rng, self.config.mines) {
            self.cells[index].content = Cell::Mine;
        }
        self.count_adjacent();
        self.seeded = true;
    }

    fn count_adjacent(&mut self) {
        let width = self.config.width;
        let counts: Vec<u8> = (0..self.cells.len())
            .map(|index| self.adjacent_mines(index / width, index % width))
            .collect();
        for (cell, count) in self.cells.iter_mut().zip(counts) {
            if cell.content != Cell::Mine {
                cell.content = Cell::Adjacent(count);
            }
        }
    }

    fn adjacent_mines(&self, row: usize, col: usize) -> u8 {
        self.neighbors(row, col)
            .filter(|&(r, c)| self.is_mine(r, c))
            .count() as u8
    }

    /// Iterative flood fill over an explicit stack. Recursion would blow the
    /// wasm stack on a large blank region, and an explicit stack also makes the
    /// visit order deterministic.
    fn flood(&mut self, row: usize, col: usize) {
        let mut stack = vec![(row, col)];
        while let Some((row, col)) = stack.pop() {
            let index = self.index(row, col);
            if self.cells[index].uncovered || self.cells[index].flagged {
                continue;
            }
            self.cells[index].uncovered = true;
            if self.cells[index].content == Cell::Adjacent(0) {
                stack.extend(self.neighbors(row, col));
            }
        }
    }

    fn lose(&mut self) -> Option<GameResult> {
        for cell in self.cells.iter_mut() {
            if cell.content == Cell::Mine {
                cell.uncovered = true;
            }
        }
        Some(GameResult::Lost)
    }

    /// Won when every non-mine cell is uncovered. Flags are irrelevant, exactly
    /// as in the original game.
    fn is_cleared(&self) -> bool {
        self.cells
            .iter()
            .all(|cell| cell.uncovered || cell.content == Cell::Mine)
    }
}

/// Board plus terminal state. Transitions are pure: `apply` returns the next
/// value and never touches `self`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    pub board: Board,
    pub result: Option<GameResult>,
}

impl GameState {
    pub fn new(config: Config) -> Self {
        Self {
            board: Board::new(config),
            result: None,
        }
    }

    pub fn is_over(&self) -> bool {
        self.result.is_some()
    }

    pub fn apply<R: Rng>(&self, action: Action, rng: &mut R) -> Self {
        match action {
            Action::Restart => Self::new(self.board.config()),
            _ if self.is_over() => self.clone(),
            Action::Reveal(row, col) => self.revealed(row, col, rng),
            Action::Flag(row, col) => self.flagged(row, col),
        }
    }

    fn revealed<R: Rng>(&self, row: usize, col: usize, rng: &mut R) -> Self {
        let mut next = self.clone();
        next.result = next.board.reveal(row, col, rng);
        next
    }

    fn flagged(&self, row: usize, col: usize) -> Self {
        let mut next = self.clone();
        next.board.toggle_flag(row, col);
        next
    }
}
