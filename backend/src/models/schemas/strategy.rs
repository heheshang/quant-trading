//! 策略相关的请求/响应 DTO

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============ Strategy ============

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StrategyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    pub template_type: String,
    pub parameters: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    #[serde(alias = "template_type")]
    pub template_type: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStrategyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateStatusRequest {
    pub ids: Vec<uuid::Uuid>,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExportParams {
    pub status: Option<String>,
    #[allow(dead_code)]
    pub format: Option<String>, // currently only json is supported
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    #[serde(alias = "template_type")]
    pub template_type: Option<String>,
    pub parameters: serde_json::Value,
    /// Optional: override status on import (default: "draft")
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportBatchRequest {
    pub strategies: Vec<ImportStrategyRequest>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportBatchResponse {
    pub imported: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateInfo {
    pub id: String,
    pub template_id: Uuid, // ADR D6: deterministic UUID for this template
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_parameters: serde_json::Value,
    pub parameter_schema: Vec<ParameterDef>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ParameterDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub label: String,
    pub description: String,
    pub default: serde_json::Value,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub options: Option<Vec<String>>,
}
