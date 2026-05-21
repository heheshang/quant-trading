pub mod config;
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod state;
pub mod strategies;
pub mod utils;

pub use models::{backtest, schemas};

use config::Config;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);
