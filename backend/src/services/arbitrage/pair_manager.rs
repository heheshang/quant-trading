//! 套利对管理器
//! 管理套利对的配置 CRUD

use crate::db::models::*;
use crate::utils::error::AppError;
use crate::schema::arbitrage_pairs;
use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;

pub struct PairManager;

impl PairManager {
    /// 创建套利对
    pub fn create(
        conn: &mut PgConnection,
        pair_type: &str,
        symbol_a: &str,
        symbol_b: &str,
        exchange: &str,
        spread_entry_threshold: Decimal,
        spread_exit_threshold: Decimal,
        max_position_size: Decimal,
        calculation_mode: &str,
        correlation_threshold: Option<Decimal>,
        z_score_entry: Option<Decimal>,
        z_score_exit: Option<Decimal>,
    ) -> Result<ArbitragePair, AppError> {
        let new_pair = NewArbitragePair {
            pair_type,
            symbol_a,
            symbol_b,
            exchange,
            status: "active",
            spread_entry_threshold,
            spread_exit_threshold,
            max_position_size,
            calculation_mode,
            correlation_threshold,
            z_score_entry,
            z_score_exit,
        };

        diesel::insert_into(arbitrage_pairs::table)
            .values(&new_pair)
            .get_result(conn)
            .map_err(AppError::Database)
    }

    /// 列出所有活跃套利对
    pub fn list_active(conn: &mut PgConnection) -> Result<Vec<ArbitragePair>, AppError> {
        arbitrage_pairs::table
            .filter(arbitrage_pairs::status.eq("active"))
            .order(arbitrage_pairs::id.desc())
            .load(conn)
            .map_err(AppError::Database)
    }

    /// 获取单个套利对
    pub fn get(conn: &mut PgConnection, pair_id: i32) -> Result<ArbitragePair, AppError> {
        arbitrage_pairs::table
            .filter(arbitrage_pairs::id.eq(pair_id))
            .first(conn)
            .map_err(AppError::Database)
    }

    /// 更新套利对配置
    pub fn update(
        conn: &mut PgConnection,
        pair_id: i32,
        pair_type: Option<&str>,
        symbol_a: Option<&str>,
        symbol_b: Option<&str>,
        exchange: Option<&str>,
        status: Option<&str>,
        spread_entry_threshold: Option<Decimal>,
        spread_exit_threshold: Option<Decimal>,
        max_position_size: Option<Decimal>,
        calculation_mode: Option<&str>,
        correlation_threshold: Option<Decimal>,
        z_score_entry: Option<Decimal>,
        z_score_exit: Option<Decimal>,
    ) -> Result<ArbitragePair, AppError> {
        let target = arbitrage_pairs::table.filter(arbitrage_pairs::id.eq(pair_id));
        
        diesel::update(target)
            .set((
                pair_type.map(|v| arbitrage_pairs::pair_type.eq(v)).unwrap_or_else(|| arbitrage_pairs::pair_type.eq(arbitrage_pairs::pair_type)),
                symbol_a.map(|v| arbitrage_pairs::symbol_a.eq(v)).unwrap_or_else(|| arbitrage_pairs::symbol_a.eq(arbitrage_pairs::symbol_a)),
                symbol_b.map(|v| arbitrage_pairs::symbol_b.eq(v)).unwrap_or_else(|| arbitrage_pairs::symbol_b.eq(arbitrage_pairs::symbol_b)),
                exchange.map(|v| arbitrage_pairs::exchange.eq(v)).unwrap_or_else(|| arbitrage_pairs::exchange.eq(arbitrage_pairs::exchange)),
                status.map(|v| arbitrage_pairs::status.eq(v)).unwrap_or_else(|| arbitrage_pairs::status.eq(arbitrage_pairs::status)),
                spread_entry_threshold.map(|v| arbitrage_pairs::spread_entry_threshold.eq(v)).unwrap_or_else(|| arbitrage_pairs::spread_entry_threshold.eq(arbitrage_pairs::spread_entry_threshold)),
                spread_exit_threshold.map(|v| arbitrage_pairs::spread_exit_threshold.eq(v)).unwrap_or_else(|| arbitrage_pairs::spread_exit_threshold.eq(arbitrage_pairs::spread_exit_threshold)),
                max_position_size.map(|v| arbitrage_pairs::max_position_size.eq(v)).unwrap_or_else(|| arbitrage_pairs::max_position_size.eq(arbitrage_pairs::max_position_size)),
                calculation_mode.map(|v| arbitrage_pairs::calculation_mode.eq(v)).unwrap_or_else(|| arbitrage_pairs::calculation_mode.eq(arbitrage_pairs::calculation_mode)),
                correlation_threshold.map(|v| arbitrage_pairs::correlation_threshold.eq(v)).unwrap_or_else(|| arbitrage_pairs::correlation_threshold.eq(arbitrage_pairs::correlation_threshold)),
                z_score_entry.map(|v| arbitrage_pairs::z_score_entry.eq(v)).unwrap_or_else(|| arbitrage_pairs::z_score_entry.eq(arbitrage_pairs::z_score_entry)),
                z_score_exit.map(|v| arbitrage_pairs::z_score_exit.eq(v)).unwrap_or_else(|| arbitrage_pairs::z_score_exit.eq(arbitrage_pairs::z_score_exit)),
                arbitrage_pairs::updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
            .map_err(AppError::Database)
    }

    /// 删除（软删除，设置 status = inactive）
    pub fn delete(conn: &mut PgConnection, pair_id: i32) -> Result<(), AppError> {
        diesel::update(arbitrage_pairs::table.filter(arbitrage_pairs::id.eq(pair_id)))
            .set((
                arbitrage_pairs::status.eq("inactive"),
                arbitrage_pairs::updated_at.eq(Utc::now()),
            ))
            .execute(conn)
            .map_err(AppError::Database)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // PairManager 依赖数据库，单元测试需要 Mock 或 integration test
    // 核心逻辑已通过 spread_calculator 和 signal_generator 的测试覆盖
}
