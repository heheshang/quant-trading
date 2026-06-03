//! Workers — background consumers for MQ queues.
//!
//! Concrete workers (ai_prediction_worker / notification_worker /
//! risk_log_worker) are defined in their own files. This module is
//! kept as a thin re-export so `pub mod workers;` in lib.rs resolves
//! even when individual workers are not yet wired up.
