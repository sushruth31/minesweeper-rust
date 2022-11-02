//! Board configuration, resolved once at start-up.
//!
//! A browser has no process environment, so the variables below are read from
//! the *build* environment via [`option_env!`] and baked into the wasm module.
//! Parsing still happens at runtime so a bad value produces a named, visible
//! error instead of a silently clamped default.

use std::fmt;

/// The first click and its eight neighbours are guaranteed mine-free, so a
/// board must keep that many cells in reserve.
pub const SAFE_REGION: usize = 9;

const DEFAULT_WIDTH: usize = 10;
const DEFAULT_HEIGHT: usize = 10;
const DEFAULT_MINES: usize = 15;

const WIDTH_VAR: &str = "MINESWEEPER_WIDTH";
const HEIGHT_VAR: &str = "MINESWEEPER_HEIGHT";
const MINES_VAR: &str = "MINESWEEPER_MINES";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub width: usize,
    pub height: usize,
    pub mines: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    NotANumber { var: &'static str, value: String },
    ZeroDimension { var: &'static str },
    TooManyMines { mines: usize, capacity: usize },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber { var, value } => {
                write!(f, "{var} must be a non-negative integer, got {value:?}")
            }
            Self::ZeroDimension { var } => write!(f, "{var} must be at least 1"),
            Self::TooManyMines { mines, capacity } => write!(
                f,
                "{MINES_VAR} is {mines} but only {capacity} cells can hold a mine \
                 (the first click and its 8 neighbours stay clear)"
            ),
        }
    }
}

impl Config {
    /// Validates dimensions and mine count, naming the offending variable.
    pub fn new(width: usize, height: usize, mines: usize) -> Result<Self, ConfigError> {
        if width == 0 {
            return Err(ConfigError::ZeroDimension { var: WIDTH_VAR });
        }
        if height == 0 {
            return Err(ConfigError::ZeroDimension { var: HEIGHT_VAR });
        }
        let capacity = (width * height).saturating_sub(SAFE_REGION);
        match mines > capacity {
            true => Err(ConfigError::TooManyMines { mines, capacity }),
            false => Ok(Self {
                width,
                height,
                mines,
            }),
        }
    }

    /// Resolves the board from the build environment, falling back to a 10x10
    /// grid with 15 mines when a variable is unset.
    pub fn from_build_env() -> Result<Self, ConfigError> {
        let width = parse(WIDTH_VAR, option_env!("MINESWEEPER_WIDTH"), DEFAULT_WIDTH)?;
        let height = parse(
            HEIGHT_VAR,
            option_env!("MINESWEEPER_HEIGHT"),
            DEFAULT_HEIGHT,
        )?;
        let mines = parse(MINES_VAR, option_env!("MINESWEEPER_MINES"), DEFAULT_MINES)?;
        Self::new(width, height, mines)
    }

    pub fn cells(&self) -> usize {
        self.width * self.height
    }
}

fn parse(var: &'static str, raw: Option<&str>, fallback: usize) -> Result<usize, ConfigError> {
    let Some(value) = raw else {
        return Ok(fallback);
    };
    value.trim().parse().map_err(|_| ConfigError::NotANumber {
        var,
        value: value.to_owned(),
    })
}
