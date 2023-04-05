#![doc = include_str!("../README.md")]
//! # Architecture
//!
//! * A [`SourceMap`] is used to load and store contents associated with [`Origin`]s,
//!   producing [`SourceIndex`] values denoting the sources.
//! * [`Input`] wrappers are created with [`SourceMap::input`] and used to traverse
//!   source contents, taking [`Offset`] positions.
//! * On error, [`Offset`] positions can be used to construct [`SourceError`]s associated
//!   with those offsets.
//! * The [`SourceError`] values can be turned into [`ContextError`] values which carry
//!   the contents associated with the context and can print full diagnostic output with
//!   [`ContextError::display_with_context`].
//! * You can also construct [`ContextError`] values with multiple error origins by passing
//!   [`ContextErrorOrigin`] values to [`ContextError::with_origins`] to build errors that
//!   involve multiple origins, like conflicts.

pub use map::*;
pub use error::*;
pub use input::*;
pub use helpers::*;


mod display;
mod map;
mod error;
mod input;
mod helpers;