//! Flightdeck library: the egui application, its places, viewport, MCP server and helpers.
//! `main.rs` is a thin launcher so tests can drive the app headlessly.

pub mod app;
pub mod commands;
pub mod history;
pub mod inspector;
pub mod mcp;
pub mod net;
pub mod places;
pub mod query;
pub mod settings;
pub mod state;
pub mod theme;
pub mod util;
pub mod viewport;
pub mod widgets;
