//! Atoms: the smallest stateless building blocks, styled only from `ui::theme`.

mod avatar;
mod badge;
mod button;
mod checkbox;
mod chip;
mod icon;
mod kbd;
mod label;
mod nav_item;
mod pill;
mod separator;
mod skeleton;
mod swatch;
mod switch;
mod tooltip;

pub use avatar::Avatar;
pub use badge::{Badge, BadgeVariant};
pub use button::{Button, ButtonVariant, Size};
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use icon::{Icon, IconName};
pub use kbd::Kbd;
pub use label::{ErrorText, SectionLabel};
pub use nav_item::NavItem;
pub use pill::Pill;
pub use separator::Separator;
pub use skeleton::Skeleton;
pub use swatch::Swatch;
pub use switch::Switch;
pub use tooltip::tooltip;
