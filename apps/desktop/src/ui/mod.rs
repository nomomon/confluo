//! App-agnostic UI kit: theme tokens, atoms and molecules.
//!
//! Nothing in here may import `crate::features`, `crate::state` or `gv_engine`.

pub mod atoms;
pub mod molecules;
pub mod theme;

use gpui::{App, ClickEvent, Window};

/// Boxed click callback shared by interactive components.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
