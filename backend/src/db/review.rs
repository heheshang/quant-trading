use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Review status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    PendingReview,
    Approved,
    Rejected,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewStatus::PendingReview => "pending_review",
            ReviewStatus::Approved => "approved",
            ReviewStatus::Rejected => "rejected",
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_review" => Some(ReviewStatus::PendingReview),
            "approved" => Some(ReviewStatus::Approved),
            "rejected" => Some(ReviewStatus::Rejected),
            _ => None,
        }
    }
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl serde::Serialize for ReviewStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ReviewStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid review status: {}", s)))
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "strategy_reviews")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// References the strategy under review
    #[sea_orm(unique)]
    pub strategy_id: Uuid,
    /// Review status: pending_review | approved | rejected
    #[sea_orm(column_type = "String(StringLen::N(32))")]
    pub review_status: String,
    pub rejection_reason: Option<String>,
    pub submitted_at: Option<DateTimeUtc>,
    pub reviewed_at: Option<DateTimeUtc>,
    /// The admin/user who reviewed this strategy
    pub reviewed_by: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Helper: get ReviewStatus from Model
impl Model {
    pub fn status(&self) -> Option<ReviewStatus> {
        ReviewStatus::from_str(&self.review_status)
    }
}
