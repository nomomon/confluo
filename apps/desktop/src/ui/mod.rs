//! App-agnostic UI kit: theme tokens, motion, atoms and molecules.
//!
//! Nothing in here may import `crate::features`, `crate::state` or `gv_engine`.

pub mod atoms;
pub mod layer;
pub mod molecules;
pub mod motion;
pub mod scroll;
pub mod theme;

use gpui::{App, ClickEvent, MouseDownEvent, Window};
use std::rc::Rc;

/// Boxed click callback shared by interactive components.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Boxed mouse-down callback (right clicks, dismiss on outside press).
pub type MouseDownHandler = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// Shared value-change callback (switches, selects, sliders, tabs).
pub type ChangeHandler<T> = Rc<dyn Fn(&T, &mut Window, &mut App) + 'static>;
