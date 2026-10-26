//! BPMN 2.0 parsing for Flightdeck: the process model plus its diagram interchange (DI)
//! geometry and all Camunda extension data, ready for rendering and inspection.

pub mod diff;
pub mod model;
pub mod parse;
pub mod search;
pub mod text;

pub use model::*;
pub use parse::{parse, ParseError};
