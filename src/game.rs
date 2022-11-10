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
        let cell = self.cell(row, col);
        if cell.flagged || cell.uncovered {
            return None;
        }
        // Seeded only once the click is known to reveal something, or a click
        // rejected above would spend the first-click guarantee on nothing.
        if !self.seeded {
            self.seed((row, col), rng);
        }
        if self.is_mine(row, col) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xC0FFEE)
    }

    /// Builds a fully seeded board from a picture: `*` is a mine, `.` is not.
    fn layout(rows: &[&str]) -> Board {
        let cells: Vec<CellState> = rows.iter().flat_map(|row| row.bytes()).map(cell).collect();
        let mines = cells.iter().filter(|c| c.content == Cell::Mine).count();
        let config = Config::new(rows[0].len(), rows.len(), 0).expect("test layout dimensions");
        let mut board = Board::new(config);
        board.config = Config { mines, ..config };
        board.cells = cells;
        board.count_adjacent();
        board.seeded = true;
        board
    }

    fn cell(byte: u8) -> CellState {
        let content = match byte {
            b'*' => Cell::Mine,
            _ => Cell::default(),
        };
        CellState {
            content,
            ..CellState::default()
        }
    }

    fn covered(board: &Board, row: usize, col: usize) -> bool {
        !board.cell(row, col).uncovered
    }

    #[test]
    fn neighbors_exclude_the_cell_itself() {
        let board = layout(&["...", "...", "..."]);
        assert!(!board.neighbors(1, 1).any(|position| position == (1, 1)));
    }

    #[test]
    fn neighbor_counts_shrink_at_edges_and_corners() {
        let board = layout(&["...", "...", "..."]);
        assert_eq!(board.neighbors(1, 1).count(), 8);
        assert_eq!(board.neighbors(0, 1).count(), 5);
        assert_eq!(board.neighbors(0, 0).count(), 3);
        assert_eq!(board.neighbors(2, 2).count(), 3);
    }

    #[test]
    fn adjacency_counts_do_not_wrap_around_the_row_boundary() {
        // Without row-major bounds checks, (0,2) and (1,0) would look adjacent.
        let board = layout(&["..*", "...", "..."]);
        assert_eq!(board.cell(1, 0).content, Cell::Adjacent(0));
        assert_eq!(board.cell(1, 1).content, Cell::Adjacent(1));
        assert_eq!(board.cell(0, 1).content, Cell::Adjacent(1));
    }

    #[test]
    fn a_cell_touching_every_neighbor_counts_all_eight() {
        let board = layout(&["***", "*.*", "***"]);
        assert_eq!(board.cell(1, 1).content, Cell::Adjacent(8));
    }

    #[test]
    fn flood_fill_reveals_the_numbered_border_but_never_steps_past_it() {
        let mut board = layout(&["....", "....", "*...", "...."]);
        assert_eq!(board.reveal(0, 3, &mut rng()), None);
        assert!(!covered(&board, 0, 0), "the whole blank region opens");
        assert!(!covered(&board, 1, 0), "its numbered border opens with it");
        assert!(covered(&board, 2, 0), "the mine stays covered");
        assert!(covered(&board, 3, 0), "so does the number behind the mine");
    }

    #[test]
    fn flood_fill_does_not_cross_a_numbered_cell() {
        let mut board = layout(&["...", ".*.", "..."]);
        assert_eq!(board.reveal(0, 0, &mut rng()), None);
        // Every non-mine cell touches the mine, so only the clicked cell opens.
        assert!(!covered(&board, 0, 0));
        assert!(covered(&board, 0, 2));
        assert!(covered(&board, 2, 2));
    }

    #[test]
    fn flood_fill_skips_flagged_cells_so_a_marked_guess_survives_the_sweep() {
        let mut board = layout(&["....", "....", "...."]);
        board.toggle_flag(2, 3);
        board.reveal(0, 0, &mut rng());
        assert!(covered(&board, 2, 3));
        assert!(!covered(&board, 2, 2));
    }

    #[test]
    fn clearing_an_empty_board_in_one_click_wins() {
        let mut board = layout(&["...", "...", "..."]);
        assert_eq!(board.reveal(1, 1, &mut rng()), Some(GameResult::Won));
    }

    #[test]
    fn revealing_a_mine_loses_and_exposes_every_other_mine() {
        let mut board = layout(&["*..", "...", "..*"]);
        assert_eq!(board.reveal(0, 0, &mut rng()), Some(GameResult::Lost));
        assert!(!covered(&board, 0, 0));
        assert!(!covered(&board, 2, 2));
        assert!(covered(&board, 1, 1), "safe cells are not given away");
    }

    #[test]
    fn a_win_needs_every_safe_cell_and_is_indifferent_to_flags() {
        let mut board = layout(&["*..", "...", "..."]);
        board.toggle_flag(0, 0);
        assert_eq!(board.reveal(2, 2, &mut rng()), Some(GameResult::Won));
        assert!(covered(&board, 0, 0), "winning never uncovers the mine");
    }

    #[test]
    fn a_flagged_cell_cannot_be_revealed_until_the_flag_comes_off() {
        let mut board = layout(&["*..", "...", "..."]);
        board.toggle_flag(0, 0);
        assert_eq!(board.reveal(0, 0, &mut rng()), None);
        assert!(covered(&board, 0, 0));
        board.toggle_flag(0, 0);
        assert_eq!(board.reveal(0, 0, &mut rng()), Some(GameResult::Lost));
    }

    #[test]
    fn flagging_an_uncovered_cell_is_a_no_op() {
        let mut board = layout(&["...", "...", "..."]);
        board.reveal(1, 1, &mut rng());
        board.toggle_flag(1, 1);
        assert!(!board.cell(1, 1).flagged);
    }

    #[test]
    fn mines_remaining_tracks_flags_and_may_go_negative() {
        let mut board = layout(&["*..", "...", "..."]);
        assert_eq!(board.mines_remaining(), 1);
        board.toggle_flag(0, 0);
        assert_eq!(board.mines_remaining(), 0);
        board.toggle_flag(0, 1);
        assert_eq!(board.mines_remaining(), -1);
    }

    #[test]
    fn out_of_bounds_coordinates_are_ignored_rather_than_panicking() {
        let mut board = layout(&["...", "...", "..."]);
        assert_eq!(board.reveal(3, 0, &mut rng()), None);
        board.toggle_flag(0, 99);
        assert!(board.rows().flatten().all(|cell| !cell.flagged));
    }

    #[test]
    fn seeding_places_exactly_the_requested_number_of_mines() {
        let config = Config::new(9, 9, 20).expect("valid config");
        let mut board = Board::new(config);
        board.reveal(4, 4, &mut rng());
        let mines = board
            .rows()
            .flatten()
            .filter(|cell| cell.content == Cell::Mine)
            .count();
        assert_eq!(mines, 20);
    }

    #[test]
    fn the_opening_click_and_its_eight_neighbors_are_always_mine_free() {
        let config = Config::new(4, 4, 7).expect("valid config"); // maximum density
        for seed in 0..200 {
            let mut board = Board::new(config);
            let outcome = board.reveal(1, 1, &mut StdRng::seed_from_u64(seed));
            assert_ne!(
                outcome,
                Some(GameResult::Lost),
                "seed {seed} put a mine under the click"
            );
            let safe = board.neighbors(1, 1).chain(std::iter::once((1, 1)));
            assert!(safe
                .map(|(r, c)| board.cell(r, c))
                .all(|cell| cell.content != Cell::Mine));
        }
    }

    #[test]
    fn a_click_on_a_flagged_cell_does_not_spend_first_click_safety() {
        let config = Config::new(4, 4, 7).expect("valid config"); // maximum density
        for seed in 0..200 {
            let mut board = Board::new(config);
            board.toggle_flag(0, 0);
            assert_eq!(board.reveal(0, 0, &mut StdRng::seed_from_u64(seed)), None);
            assert!(
                !board.seeded,
                "a click that revealed nothing laid the mines"
            );
            assert_ne!(
                board.reveal(3, 3, &mut StdRng::seed_from_u64(seed)),
                Some(GameResult::Lost),
                "seed {seed} put a mine under the first visible reveal"
            );
        }
    }

    #[test]
    fn mine_placement_is_reproducible_for_a_fixed_seed() {
        let config = Config::new(8, 8, 10).expect("valid config");
        let boards: Vec<Board> = (0..2)
            .map(|_| {
                let mut board = Board::new(config);
                board.reveal(0, 0, &mut StdRng::seed_from_u64(7));
                board
            })
            .collect();
        assert_eq!(boards[0], boards[1]);
    }

    #[test]
    fn actions_after_the_game_ends_leave_the_state_untouched() {
        let lost = GameState {
            board: layout(&["*...", "....", "....", "...."]),
            result: Some(GameResult::Lost),
        };
        assert_eq!(lost.apply(Action::Reveal(3, 3), &mut rng()), lost);
        assert_eq!(lost.apply(Action::Flag(3, 3), &mut rng()), lost);
    }

    #[test]
    fn restart_rebuilds_an_unseeded_board_with_the_same_configuration() {
        let config = Config::new(6, 6, 5).expect("valid config");
        let state = GameState::new(config).apply(Action::Reveal(0, 0), &mut rng());
        let restarted = state.apply(Action::Restart, &mut rng());
        assert_eq!(restarted, GameState::new(config));
        assert!(restarted.board.rows().flatten().all(|cell| !cell.uncovered));
    }

    #[test]
    fn a_restart_is_honoured_even_after_a_loss() {
        let state = GameState {
            board: layout(&["*..", "...", "..."]),
            result: Some(GameResult::Lost),
        };
        assert!(!state.apply(Action::Restart, &mut rng()).is_over());
    }
}
