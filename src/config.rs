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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_variables_fall_back_to_the_default_board() {
        assert_eq!(parse(WIDTH_VAR, None, 10), Ok(10));
    }

    #[test]
    fn the_build_environment_always_resolves_to_a_playable_board() {
        let config = Config::from_build_env().expect("build variables describe a valid board");
        assert!(config.mines + SAFE_REGION <= config.cells());
    }

    #[test]
    fn surrounding_whitespace_in_a_build_variable_is_tolerated() {
        assert_eq!(parse(WIDTH_VAR, Some(" 16 "), 10), Ok(16));
    }

    #[test]
    fn a_non_numeric_build_variable_is_rejected_by_name() {
        let error = parse(MINES_VAR, Some("lots"), 15);
        assert_eq!(
            error,
            Err(ConfigError::NotANumber {
                var: MINES_VAR,
                value: "lots".to_owned(),
            })
        );
        assert!(format!("{}", error.unwrap_err()).contains(MINES_VAR));
    }

    #[test]
    fn a_zero_dimension_is_rejected_naming_the_offending_axis() {
        assert_eq!(
            Config::new(0, 10, 1),
            Err(ConfigError::ZeroDimension { var: WIDTH_VAR })
        );
        assert_eq!(
            Config::new(10, 0, 1),
            Err(ConfigError::ZeroDimension { var: HEIGHT_VAR })
        );
    }

    #[test]
    fn mine_count_may_not_eat_into_the_guaranteed_safe_region() {
        assert_eq!(
            Config::new(4, 4, 7),
            Ok(Config {
                width: 4,
                height: 4,
                mines: 7,
            })
        );
        assert_eq!(
            Config::new(4, 4, 8),
            Err(ConfigError::TooManyMines {
                mines: 8,
                capacity: 7,
            })
        );
    }

    #[test]
    fn a_board_smaller_than_the_safe_region_admits_no_mines() {
        assert!(Config::new(2, 2, 0).is_ok());
        assert!(Config::new(2, 2, 1).is_err());
    }
}
