//! Reusable controllers for direct manipulation, built on `motion::physics`: `Pager` (drag between snap
//! points), `SwipeGate` (wheel events to a horizontal drag), `ElasticScroll` (rubber-band overscroll)
//! and `Animated` (a value that springs to its target, for spatial transitions).

mod animated;
mod elastic;
mod gesture;
mod pager;

#[allow(unused_imports)] // kit API: spatial transitions that are not gestures
pub use animated::Animated;
pub use elastic::{ElasticScroll, Pin};
pub use gesture::{Drag, SwipeGate};
pub use pager::Pager;
