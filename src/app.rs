use gloo::console::log;
use rand;
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
}

//function to make grid of CellState
fn create_grid_state() -> Vec<Vec<CellState>> {
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
    grid
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

#[function_component(App)]
pub fn app() -> Html {
    let grid_size = use_state(|| GRID_SIZE);
    //grid state is a vec of vecs of cell states
    let grid_state = use_state(create_grid_state);
    let grid_state_inner = grid_state.to_vec();
    //state to see if flag or uncover is selected
    let action = use_state(|| Action::Uncover);

    //function to uncover a cell given a row and column
    let on_oncover: Callback<(usize, usize)> = {
        let grid_state = grid_state.clone();
        Callback::from(move |(row, col)| {
            //log row and column
            log!(row, col);
            let grid_state = grid_state.to_vec();
            //get cell state at row and column
            let cell_state = grid_state[row as usize][col as usize];
            //if the cell is uncovered return
            if cell_state.uncovered {
                return;
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
        Callback::from(move |(row, col)| match *action {
            //if action uncover call on uncover
            Action::Uncover => on_oncover.emit((row, col)),
            //if action flag call on flag
            Action::Flag => on_flag((row, col)),
        })
    };

    let rows = grid_state_inner
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells = row.iter().enumerate().map(|(j, cell)| {
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
            });
            html! {
                <div class="row">
                {for cells}
                </div>
            }
        })
        .collect::<Vec<_>>();
    //create rows of grid size
    html! {
        <main>
        <h1>{ "Minesweeeper Rust!" }</h1>
        { for rows }
        </main>
    }
}
