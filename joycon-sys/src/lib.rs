//! Helper crate for interacting with a JoyCon and Switch Pro Controller via HID.
//!
//! The main structs are [InputReport](input/struct.InputReport.html) and
//! [OutputReport](input/struct.OutputReport.html).

#[macro_use]
extern crate num_derive;

mod common;
pub mod input;
pub mod output;

pub use common::*;
pub use input::InputReport;
pub use output::OutputReport;
