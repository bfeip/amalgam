//#![warn(missing_debug_implementations)]
//#![warn(missing_docs)]
#![allow(dead_code)]

extern crate cpal;

pub mod error;
pub mod util;
pub mod prelude;

mod midi;
pub mod note;
mod clock;
pub mod module;
pub mod output;
pub mod synth;

pub use synth::Synth;
pub use error::{SynthError, SynthResult};