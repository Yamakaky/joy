//! Helper crate for interacting with a JoyCon and Switch Pro Controller via HID.
//!
//! The main structs are [InputReport](input/struct.InputReport.html) and
//! [OutputReport](output/struct.OutputReport.html).

#[macro_use]
extern crate num_derive;

pub mod common;
pub mod imu;
pub mod input;
pub mod light;
pub mod mcu;
pub mod output;
pub mod accessory;
pub mod spi;

pub use common::*;
pub use input::InputReport;
pub use output::OutputReport;
