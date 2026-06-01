// ============ Template Registry ============
//
// Builds the in-memory list of built-in `StrategyTemplate` implementations
// and provides DTO conversion helpers used by `crud`.
use super::common::StrategyTemplate;
use super::templates_impls::{
    AtrStopTemplate, BollingerTemplate, DoubleBollingerTemplate, IchimokuTemplate, KeltnerTemplate,
    MacdTemplate, MaCrossoverTemplate, MeanReversionTemplate, RsiTemplate, TripleMaTemplate,
};
use crate::db::strategy;
use crate::models::schemas::{StrategyResponse, TemplateInfo};
use crate::utils::error::AppError;
use uuid::Uuid;

pub fn get_all_templates() -> Vec<Box<dyn StrategyTemplate>> {
    vec![
        Box::new(MaCrossoverTemplate),
        Box::new(TripleMaTemplate),
        Box::new(MacdTemplate),
        Box::new(BollingerTemplate),
        Box::new(RsiTemplate),
        Box::new(KeltnerTemplate),
        Box::new(AtrStopTemplate),
        Box::new(MeanReversionTemplate),
        Box::new(IchimokuTemplate),
        Box::new(DoubleBollingerTemplate),
    ]
}

pub fn get_template(id: &str) -> Option<Box<dyn StrategyTemplate>> {
    get_all_templates().into_iter().find(|t| t.id() == id)
}

pub(crate) fn template_to_info(t: Box<dyn StrategyTemplate>) -> TemplateInfo {
    TemplateInfo {
        id: t.id().to_string(),
        template_id: template_type_to_uuid(t.id()),
        name: t.name().to_string(),
        description: t.description().to_string(),
        category: t.category().to_string(),
        default_parameters: t.default_parameters(),
        parameter_schema: t.parameter_schema(),
    }
}

// ============ Status Validation ============

pub(crate) fn validate_status_transition(current: &str, next: &str) -> Result<(), AppError> {
    let valid = matches!(
        (current, next),
        ("draft", "active") | ("active", "paused") | ("paused", "active") | ("paused", "stopped")
    );
    if !valid {
        return Err(AppError::Validation(format!(
            "Invalid status transition: {} -> {}",
            current, next
        )));
    }
    Ok(())
}

// Mapping from template_type string to a consistent UUID for ADR D6 compliance.
// In production this could come from a database templates table.
pub(crate) fn template_type_to_uuid(template_type: &str) -> Uuid {
    // Use a simple FNV-style hash to derive octets, then build v4-style UUID
    let bytes = template_type.as_bytes();
    let mut hash: u64 = 0xcbf29ce84222374fu64;
    for &b in bytes {
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= b as u64;
    }
    let (d1, d2) = ((hash >> 48) as u32, (hash >> 32) as u16);
    let d3 = (hash >> 16) as u16;
    let d4: [u8; 8] = [
        (hash >> 8) as u8,
        hash as u8,
        ((hash >> 56) ^ 0x40) as u8, // set version = 4
        ((hash >> 48) ^ 0x80) as u8, // set variant
        ((hash >> 40) & 0xff) as u8,
        ((hash >> 32) & 0xff) as u8,
        ((hash >> 24) & 0xff) as u8,
        ((hash >> 16) & 0xff) as u8,
    ];
    Uuid::from_fields(d1, d2, d3, &d4)
}

pub(crate) fn model_to_response(m: strategy::Model) -> StrategyResponse {
    StrategyResponse {
        id: m.id,
        user_id: m.user_id,
        name: m.name,
        description: m.description,
        symbol: m.symbol,
        timeframe: m.timeframe,
        strategy_type: m.strategy_type,
        template_id: template_type_to_uuid(&m.template_type),
        template_type: m.template_type,
        parameters: m.parameters,
        status: m.status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

