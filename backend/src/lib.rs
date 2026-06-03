pub mod config;
pub mod db;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod mq;
pub mod models;
pub mod observability;
pub mod services;
pub mod state;
pub mod strategies;
pub mod utils;
pub mod workers;

pub use models::{backtest, schemas};

use config::Config;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);
