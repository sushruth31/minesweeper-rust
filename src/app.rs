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
    flagged: bool,
}

impl Default for CellState {
    fn default() -> Self {
        CellState {
            content: Cell::Empty,
            uncovered: false,
            flagged: false,
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

fn coin_toss() -> bool {
    rand::random::<f64>() > 0.7
}

impl Board {
    //create a new board
    fn new() -> Self {
        let mut grid = Vec::new();
        for _ in 0..GRID_SIZE {
            let mut row = Vec::new();
            for _ in 0..GRID_SIZE {
                if coin_toss() {
                    row.push(CellState {
                        content: Cell::Mine,
                        ..CellState::default()
                    });
                } else {
                    //push the default cell state
                    row.push(CellState::default());
                }
            }
            grid.push(row);
        }
        let mut board = Board {
            cells: grid.to_vec(),
        };
        //loop through each cell and set the value
        for (i, row) in grid.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if cell.is_mine() {
                    continue;
                }
                //get the neihbords of the cell and filter for mines and count them
                let mine_count = board.get_neighbors(i, j).iter().fold(0, |mut acc, cell| {
                    if board.is_cell_mine(cell.0, cell.1) {
                        acc + 1
                    } else {
                        acc
                    }
                });

                //if count is greater than 0 set the value of the cell to the count
                if mine_count > 0 {
                    board.cells[i][j].content = Cell::Value(mine_count as i32)
                }
            }
        }
        //return new Board
        board
    }

    fn is_cell_flagged(&self, row: usize, col: usize) -> bool {
        self.cells[row][col].flagged
    }

    fn add_flag(&self, row: usize, col: usize) -> Self {
        //copy the board
        let mut board = self.deref().clone();
        board[row][col].flagged = true;
        return Board {
            cells: board.to_vec(),
        };
    }

    fn remove_flag(&self, row: usize, col: usize) -> Self {
        //copy the board
        let mut board = self.deref().clone();
        board[row][col].flagged = false;
        log!("remove flag");
        return Board {
            cells: board.to_vec(),
        };
    }

    //uncover a cell given a row and column and hashset of empty cells visited
    fn uncover(&self, row: usize, col: usize) -> Result<Board, Board> {
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        //add the current cell to the visited set
        visited.insert((row, col));
        let mut cells = self.deref().to_vec();
        //while the hashset of visited cells is not empty
        while !visited.is_empty() {
            //get the first element in the hashset and remove it
            let (row, col) = visited.iter().next().unwrap().clone();
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
                        //if the neighbor is not uncovered is empty and not visited add it to the visited set
                        visited.insert((row, col));
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
            //if row is 0 and i is 0 or row is grid size - 1 and i is 2 skip
            if (row == 0 && i == 0) || (row == GRID_SIZE - 1 && i == 2) {
                continue;
            }
            for j in 0..3 {
                //if col is 0 and j is 0 or col is grid size - 1 and j is 2 skip
                if (col == 0 && j == 0) || (col == GRID_SIZE - 1 && j == 2) {
                    continue;
                }
                let row_attempt = row + i - 1;
                //get column
                let col_attempt = col + j - 1;
                //push the cell to the neighbors vec
                neighbors.push((row_attempt, col_attempt));
            }
        }

        //return the neighbors
        neighbors
    }

    fn is_out_of_bounds(&self, row: usize, col: usize) -> bool {
        row >= GRID_SIZE || col >= GRID_SIZE
    }

    fn is_cell_empty(&self, row: usize, col: usize) -> bool {
        self.cells[row][col].is_empty()
    }

    fn is_cell_mine(&self, row: usize, col: usize) -> bool {
        self.cells[row][col].is_mine()
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
            //if the cell is flagged return
            if grid_state.is_cell_flagged(row, col) {
                return;
            }
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
            //if the cell is flagged remove the flag
            if grid_state.is_cell_flagged(row, col) {
                grid_state.set(grid_state.remove_flag(row, col));
            } else {
                //if the cell is not flagged add the flag
                grid_state.set(grid_state.add_flag(row, col));
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
                                //if cell is not uncovered and is flagged show flag
                                if cell_inner.flagged {
                                    html!{<div {onclick} class="covered">{"🚩"}</div>}
                                } else {
                                html!{
                                    <div {onclick} class="covered"></div>
                                }
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

    //initial class for uncover action
    let mut uncover_class = "".to_string();
    //if the action is uncover then append selected to the class
    if *action == Action::Uncover {
        uncover_class.push_str(" selected");
    }
    //initial class for flag action
    let mut flag_class = "".to_string();
    //if the action is flag then append selected to the class
    if *action == Action::Flag {
        flag_class.push_str(" selected");
    }

    //callback for action click to set the action
    let on_action_click = {
        let action = action.clone();
        Callback::from(move |_: MouseEvent| {
            //if the action is uncover then set the action to flag else set the action to uncover
            if *action == Action::Uncover {
                action.set(Action::Flag);
            } else {
                action.set(Action::Uncover);
            }
        })
    };

    let on_action_click_clone = on_action_click.clone();

    html! {
        <main>
        <h1>{ "Minesweeeper Rust!" }</h1>
        //add icons for flag and uncover
        <div class="actions">
            <div onclick={on_action_click} class={uncover_class.to_string()}>
                <p>{ "Uncover" }</p>
            </div>
            <div onclick={on_action_click_clone} class={flag_class.to_string()}>
                <p>{ "Flag" }</p>
            </div>
        </div>
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
