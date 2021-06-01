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

pub use error::{SynthError, SynthResult};