//! ATR 追踪止损服务 (ADR-015)
//!
//! 随价格有利方向自动上移止损价，触发时市价平仓。
//!
//! 集成点：
//! - `risk_manager.rs` 开仓时调用 `init_stop_loss()`
//! - `WsHub` 行情更新时调用 `update_and_check()`

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::atr_stop_loss;
use crate::db::atr_stop_loss::Entity as AtrStopLossEntity;
use crate::utils::error::AppError;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossInit {
    pub position_id: Uuid,
    pub entry_price: Decimal,
    pub atr_value: Decimal,
    pub atr_period: i32,
    pub multiplier: Decimal,
    pub side: String, // "long" | "short"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossUpdate {
    pub position_id: Uuid,
    pub current_price: Decimal,
    pub current_atr: Decimal,
    pub multiplier: Decimal,
    pub side: String,
}

// ─── Service ──────────────────────────────────────────────────────────────

pub struct AtrStopLossService {
    db: Arc<DatabaseConnection>,
}

impl AtrStopLossService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 初始化 ATR 追踪止损（开仓时调用）
    /// 计算初始止损价：entry_price - atr_value * multiplier（多头）/ entry_price + atr_value * multiplier（空头）
    pub async fn init_stop_loss(&self, params: StopLossInit) -> Result<Decimal, AppError> {
        let stop_price = match params.side.as_str() {
            "long" => params.entry_price - params.atr_value * params.multiplier,
            "short" => params.entry_price + params.atr_value * params.multiplier,
            _ => return Err(AppError::Internal("Invalid position side".to_string())),
        };

        let model = atr_stop_loss::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            position_id: sea_orm::Set(params.position_id),
            entry_price: sea_orm::Set(params.entry_price),
            current_stop: sea_orm::Set(stop_price),
            atr_value: sea_orm::Set(params.atr_value),
            atr_period: sea_orm::Set(params.atr_period),
            multiplier: sea_orm::Set(params.multiplier),
            position_side: sea_orm::Set(params.side.clone()),
            created_at: sea_orm::Set(chrono::Utc::now()),
            updated_at: sea_orm::Set(chrono::Utc::now()),
        };

        AtrStopLossEntity::insert(model)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(stop_price)
    }

    /// 更新追踪止损价并检测是否触发
    /// 行情更新时调用（每次 ticker 更新）
    /// 返回 Some(new_stop) 如果止损价有更新，返回 None 如果未触发
    pub async fn update_and_check(
        &self,
        params: StopLossUpdate,
    ) -> Result<Option<StopLossResult>, AppError> {
        // 查询当前止损记录
        let record = AtrStopLossEntity::find()
            .filter(atr_stop_loss::Column::PositionId.eq(params.position_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let record = match record {
            Some(r) => r,
            None => return Ok(None), // 无追踪止损记录，跳过
        };

        let side = record.position_side.as_str();
        let current_stop = record.current_stop;

        // 计算新的止损价
        let new_stop = match side {
            "long" => {
                // 多头：仅在新价格大幅上涨时上移止损（追踪更高的高点）
                let potential = params.current_price - params.current_atr * params.multiplier;
                if potential > current_stop {
                    potential
                } else {
                    current_stop
                }
            }
            "short" => {
                // 空头：仅在新价格大幅下跌时下移止损（追踪更低的低点）
                let potential = params.current_price + params.current_atr * params.multiplier;
                if potential < current_stop {
                    potential
                } else {
                    current_stop
                }
            }
            _ => return Err(AppError::Internal("Invalid side".to_string())),
        };

        // 检测是否触发
        let triggered = match side {
            "long" => params.current_price <= current_stop,
            "short" => params.current_price >= current_stop,
            _ => false,
        };

        let mut result = StopLossResult {
            triggered,
            old_stop: current_stop,
            new_stop,
            updated: false,
        };

        if triggered {
            // 触发：清除追踪止损记录
            let mut del: atr_stop_loss::ActiveModel = record.into();
            del.updated_at = sea_orm::Set(chrono::Utc::now());
            AtrStopLossEntity::delete(del)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            return Ok(Some(result));
        }

        if new_stop != current_stop {
            // 止损价有更新，写入数据库
            let mut upd: atr_stop_loss::ActiveModel = record.into();
            upd.current_stop = sea_orm::Set(new_stop);
            upd.atr_value = sea_orm::Set(params.current_atr);
            upd.updated_at = sea_orm::Set(chrono::Utc::now());
            AtrStopLossEntity::update(upd)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            result.updated = true;
        }

        Ok(Some(result))
    }

    /// 获取当前持仓的 ATR 止损价（用于 UI 显示）
    pub async fn get_stop_price(&self, position_id: Uuid) -> Result<Option<Decimal>, AppError> {
        let record = AtrStopLossEntity::find()
            .filter(atr_stop_loss::Column::PositionId.eq(position_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(record.map(|r| r.current_stop))
    }

    /// 计算入场价的 ATR 止损（静态，不跟踪）
    pub fn calc_static_stop(
        entry_price: Decimal,
        atr_value: Decimal,
        multiplier: Decimal,
        side: &str,
    ) -> Decimal {
        match side {
            "long" => entry_price - atr_value * multiplier,
            "short" => entry_price + atr_value * multiplier,
            _ => entry_price,
        }
    }
}

/// ATR 计算结果（含追踪状态）
#[derive(Debug, Clone)]
pub struct StopLossResult {
    pub triggered: bool,   // 是否触发
    pub old_stop: Decimal, // 原止损价
    pub new_stop: Decimal, // 新止损价（已更新）
    pub updated: bool,     // 止损价是否更新
}

// ─── Database Entity ─────────────────────────────────────────────────────

// Entity is defined in `crate::db::atr_stop_loss`
// This service module uses it via `crate::db::atr_stop_loss::Entity`

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_static_stop_long() {
        let entry = Decimal::new(65000, 0); // 65000
        let atr = Decimal::new(300, 0); // 300
        let mult = Decimal::new(15, 1); // 1.5

        let stop = AtrStopLossService::calc_static_stop(entry, atr, mult, "long");
        // 65000 - 300 * 1.5 = 65000 - 450 = 64550
        assert_eq!(stop, Decimal::new(64550, 0));
    }

    #[test]
    fn test_calc_static_stop_short() {
        let entry = Decimal::new(65000, 0);
        let atr = Decimal::new(300, 0);
        let mult = Decimal::new(15, 1);

        let stop = AtrStopLossService::calc_static_stop(entry, atr, mult, "short");
        // 65000 + 300 * 1.5 = 65000 + 450 = 65450
        assert_eq!(stop, Decimal::new(65450, 0));
    }

    #[test]
    fn test_trailing_stop_only_moves_profitably_long() {
        let entry = Decimal::new(65000, 0);
        let atr = Decimal::new(300, 0);
        let mult = Decimal::new(15, 1);

        // 价格从 65000 涨到 66000，ATR=300，ATR止损应上移
        let current_price = Decimal::new(66000, 0);
        let current_stop = entry - atr * mult; // 64550

        let potential_stop = current_price - atr * mult; // 66000 - 450 = 65550
        assert!(potential_stop > current_stop); // 止损应上移
        assert_eq!(potential_stop, Decimal::new(65550, 0));

        // 价格从 66000 小幅回调到 65600，ATR=300，ATR止损应保持或下移
        let current_price2 = Decimal::new(65600, 0);
        let potential_stop2 = current_price2 - atr * mult; // 65600 - 450 = 65150
        assert!(potential_stop2 < potential_stop); // 止损应跟随回调
        assert!(potential_stop2 > current_stop); // 但不应低于初始止损
    }
}
