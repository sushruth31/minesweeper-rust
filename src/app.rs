use gloo::console::log;
use rand;
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};
use web_sys::console;
use yew::prelude::*;

const GRID_SIZE: usize = 10;

//struct for cell state

#[derive(Clone, Copy, PartialEq, Debug)]
struct CellState {
    content: Cell,
    uncovered: bool,
}

impl Default for CellState {
    fn default() -> Self {
        CellState {
            content: Cell::Empty,
            uncovered: false,
        }
    }
}

impl CellState {
    fn is_empty(&self) -> bool {
        self.content == Cell::Empty
    }

    fn is_mine(&self) -> bool {
        self.content == Cell::Mine
    }

    fn get_value(&self) -> usize {
        //if cell is value return value
        if let Cell::Value(value) = self.content {
            value as usize
        } else {
            0
        }
    }
}

//struct for vec of vec of cell state
#[derive(Default, Debug)]
struct Board {
    cells: Vec<Vec<CellState>>,
}

impl Board {
    //create a new board
    fn new() -> Self {
        let mut grid = Vec::new();
        for _ in 0..GRID_SIZE {
            let mut row = Vec::new();
            for i in 0..GRID_SIZE {
                //get a radom number between 0 and grid size
                let num_mines = rand::random::<usize>() % GRID_SIZE;
                //make a vector of length betweeen 0 and grid size and fill it with random numbers between 0 and grid size
                let mines_indicies = (0..num_mines)
                    .map(|_| rand::random::<usize>() % GRID_SIZE)
                    .collect::<Vec<_>>();
                if mines_indicies.contains(&i) {
                    row.push(CellState {
                        content: Cell::Mine,
                        uncovered: false,
                    });
                } else {
                    //push the default cell state
                    row.push(CellState::default());
                }
            }
            grid.push(row);
        }
        //return new Board
        Board { cells: grid }
    }

    //uncover a cell given a row and column and hashset of empty cells visited
    fn uncover(&self, row: usize, col: usize) -> Result<Board, Board> {
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        //add the current cell to the visited set
        visited.insert((row, col));
        let mut cells = self.deref().to_vec();
        //while the hashset of visited cells is not empty
        while !visited.is_empty() {
            //get the first element in the hashset
            let (row, col) = visited.iter().next().unwrap().clone();
            //remove the first element from the hashset
            visited.remove(&(row, col));
            //if the cell is not uncovered
            if !cells[row][col].uncovered {
                //uncover the cell
                cells[row][col].uncovered = true;
                //if cell is a bomb return error
                if cells[row][col].is_mine() {
                    return Err(Board { cells });
                }
                //if the cell is empty
                if cells[row][col].is_empty() {
                    //get the neighbors of the cell
                    let neighbors = self.get_neighbors(row, col);
                    for (row, col) in neighbors {
                        //if the neighbor is not uncovered is empty
                        if !cells[row][col].uncovered && cells[row][col].is_empty() {
                            visited.insert((row, col));
                        }
                    }
                }
            }
        }

        //return the new board
        Ok(Board { cells })
    }

    fn get_neighbors(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut neighbors = Vec::new();
        //get the neighbors of a cell
        //get the cells to the top of cell

        for i in 0..3 {
            for j in 0..3 {
                let row_attempt = if row == 0 { 0 } else { row - 1 + i };
                //push cells to the top left, top, and top right
                //if cell is not the cell itself and cell is not out out of bounds
                //get column
                let col_attempt = if col == 0 { 0 } else { col - 1 + j };
                if (row_attempt != row || col_attempt != col)
                    && !self.is_out_of_bounds(row_attempt, col_attempt)
                {
                    neighbors.push((row_attempt, col_attempt));
                }
            }
        }

        //return the neighbors
        neighbors
    }

    fn is_out_of_bounds(&self, row: usize, col: usize) -> bool {
        row >= GRID_SIZE || col >= GRID_SIZE
    }
}

//enum for grid cell
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cell {
    Value(i32),
    Empty,
    Mine,
}

//enum for flag or uncover
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    Flag,
    Uncover,
}

//emum for game result
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameResult {
    Win,
    Lose,
}

//implement deref for board
impl Deref for Board {
    type Target = Vec<Vec<CellState>>;
    fn deref(&self) -> &Self::Target {
        &self.cells
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let grid_size = use_state(|| GRID_SIZE);
    //grid state is a vec of vecs of cell states
    let grid_state = use_state(Board::new);
    let grid_state_inner = grid_state.deref();
    //state to see if flag or uncover is selected
    let action = use_state(|| Action::Uncover);
    let game_result: UseStateHandle<Option<GameResult>> = use_state(|| None);
    let game_result_inner = *game_result;

    //function to uncover a cell given a row and column
    let on_oncover: Callback<(usize, usize)> = {
        let grid_state = grid_state.clone();
        Callback::from(move |(row, col)| {
            //uncover the cell and get the new board
            let new_board = grid_state.uncover(row, col);
            //set the board
            //if newboard is ok set the board to the new board
            if let Ok(new_board) = new_board {
                grid_state.set(new_board);
            } else {
                //if new board is err set the board to the new board
                grid_state.set(new_board.unwrap_err());
                //set the game result to lose
                game_result.set(Some(GameResult::Lose));
            }
        })
    };

    //function to handle flagging a cell
    let on_flag = {
        let grid_state = grid_state.clone();
        move |(row, col)| -> () {
            let grid_state = grid_state.to_vec();
            let cell_state = grid_state[row as usize][col as usize];
            if cell_state.uncovered {
                return;
            }
        }
    };

    //oncellclick function if action is uncover then handle uncover else handle flag
    let on_cell_click: Callback<(usize, usize)> = {
        let action = action.clone();
        let game_result = game_result_inner.clone();
        Callback::from(move |(row, col)| {
            //if game result is some then return
            if game_result.is_some() {
                return;
            }
            match *action {
                //if action uncover call on uncover
                Action::Uncover => on_oncover.emit((row, col)),
                //if action flag call on flag
                Action::Flag => on_flag((row, col)),
            }
        })
    };

    let rows = grid_state_inner
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells = row
                .iter()
                .enumerate()
                .map(|(j, cell)| {
                    let cell_inner = cell.to_owned();
                    let f = on_cell_click.clone();
                    //bind on cell click to the cell
                    let onclick = Callback::from(move |_| f.emit((i, j)));
                    html! {
                        <div class="cell">
                            {if cell_inner.uncovered {
                                match cell_inner.content {
                                    Cell::Value(val) => html!{val},
                                    Cell::Mine => html!{"💣"},
                                    Cell::Empty => html!{" "},
                                }
                            } else {
                                //covered class for cell_inner
                                html!{
                                    <div {onclick} class="covered"></div>
                                }
                            }}
                        </div>
                    }
                })
                .collect::<Html>();
            html! {
                <div class="row">
                {cells}
                </div>
            }
        })
        .collect::<Vec<_>>();
    //create rows of grid size
    html! {
        <main>
        <h1>{ "Minesweeeper Rust!" }</h1>
        //if game result is lose show game over
        {if let Some(GameResult::Lose) = game_result_inner {
            html!{
                <div class="game-over">
                    {"Game Over"}
                </div>
            }
        } else {
            html!{}
        }}
        { for rows }
        </main>
    }
}
