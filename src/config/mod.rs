//! Configuration loading for oxsb.
//!
//! # Quick start
//!
//! ```no_run
//! use std::path::Path;
//! use oxsb::config::load_config;
//! use oxsb::expand::default_vars;
//!
//! let config = load_config(Path::new("~/.config/oxsb/config.yaml"), &default_vars()).unwrap();
//! ```

pub mod loader;
pub mod schema;

pub use loader::{load_config, load_config_dry};
pub use schema::Config;
