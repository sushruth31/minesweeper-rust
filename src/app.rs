//! Yew view layer.
//!
//! Contains no rules: every transition is `GameState::apply`, and the component
//! only turns the resulting board into DOM nodes.

use std::rc::Rc;

use rand::thread_rng;
use yew::prelude::*;

use crate::config::{Config, ConfigError};
use crate::game::{Action, Board, Cell, CellState, GameResult, GameState};

impl Reducible for GameState {
    type Action = Action;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        Rc::new(self.apply(action, &mut thread_rng()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Uncover,
    Flag,
}

impl Mode {
    fn action(self, row: usize, col: usize) -> Action {
        match self {
            Mode::Uncover => Action::Reveal(row, col),
            Mode::Flag => Action::Flag(row, col),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    /// Carried as a `Result` so a bad build variable is reported on the page
    /// rather than swallowed or defaulted away.
    pub config: Result<Config, ConfigError>,
}

#[function_component(App)]
pub fn app(props: &Props) -> Html {
    match &props.config {
        Ok(config) => html! { <Game config={*config} /> },
        Err(error) => html! {
            <main><p class="fatal">{ format!("configuration error: {error}") }</p></main>
        },
    }
}

#[derive(Properties, PartialEq)]
struct GameProps {
    config: Config,
}

#[function_component(Game)]
fn game(props: &GameProps) -> Html {
    let config = props.config;
    let state = use_reducer(move || GameState::new(config));
    let mode = use_state(|| Mode::Uncover);
    let on_cell = {
        let (state, mode) = (state.clone(), *mode);
        Callback::from(move |(row, col)| state.dispatch(mode.action(row, col)))
    };
    let on_restart = {
        let state = state.clone();
        Callback::from(move |_: MouseEvent| state.dispatch(Action::Restart))
    };
    html! {
        <main>
            <h1>{ "Minesweeper" }</h1>
            <div class="toolbar">
                { mode_button("Uncover", Mode::Uncover, &mode) }
                { mode_button("Flag", Mode::Flag, &mode) }
                <button onclick={on_restart} class="mode">{ "New game" }</button>
            </div>
            { status(&state) }
            { grid(&state.board, &on_cell) }
        </main>
    }
}

fn mode_button(label: &str, target: Mode, mode: &UseStateHandle<Mode>) -> Html {
    let class = match **mode == target {
        true => "mode selected",
        false => "mode",
    };
    let onclick = {
        let mode = mode.clone();
        Callback::from(move |_: MouseEvent| mode.set(target))
    };
    html! { <button {onclick} {class}>{ label }</button> }
}

fn status(state: &GameState) -> Html {
    let (class, text) = match state.result {
        Some(GameResult::Won) => ("status won", "Swept.".to_owned()),
        Some(GameResult::Lost) => ("status lost", "Boom.".to_owned()),
        None => (
            "status",
            format!("{} mines left", state.board.mines_remaining()),
        ),
    };
    html! { <p {class}>{ text }</p> }
}

fn grid(board: &Board, on_cell: &Callback<(usize, usize)>) -> Html {
    board
        .rows()
        .enumerate()
        .map(|(row, cells)| {
            let cells: Html = cells
                .iter()
                .enumerate()
                .map(|(col, cell)| cell_view(row, col, cell, on_cell))
                .collect();
            html! { <div class="row">{ cells }</div> }
        })
        .collect()
}

fn cell_view(row: usize, col: usize, cell: &CellState, on_cell: &Callback<(usize, usize)>) -> Html {
    let onclick = {
        let on_cell = on_cell.clone();
        Callback::from(move |_: MouseEvent| on_cell.emit((row, col)))
    };
    html! { <div {onclick} class={cell_class(cell)}>{ cell_face(cell) }</div> }
}

fn cell_class(cell: &CellState) -> &'static str {
    match (cell.uncovered, cell.content) {
        (false, _) => "cell covered",
        (true, Cell::Mine) => "cell mine",
        (true, _) => "cell uncovered",
    }
}

fn cell_face(cell: &CellState) -> Html {
    match (cell.uncovered, cell.flagged, cell.content) {
        (false, true, _) => html! { "\u{1F6A9}" },
        (false, false, _) => html! {},
        (true, _, Cell::Mine) => html! { "\u{1F4A3}" },
        (true, _, Cell::Adjacent(0)) => html! {},
        (true, _, Cell::Adjacent(count)) => html! { count },
    }
}
