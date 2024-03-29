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
mod output;
pub mod synth;

pub use crate::synth::Synth;
pub use error::{SynthError, SynthResult};

pub mod signal_logger;
pub use signal_logger::SignalLogger;