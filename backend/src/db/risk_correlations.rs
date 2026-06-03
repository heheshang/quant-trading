//! P1-5: `risk_correlations` SeaORM Entity
//!
//! Stores Pearson r between two symbols for one user. The (symbol_a,
//! symbol_b) pair is stored in canonical (lexicographic) order so the
//! `UNIQUE` constraint can't catch both `(A, B)` and `(B, A)`.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "risk_correlations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub user_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub symbol_a: String,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub symbol_b: String,
    /// Pearson r in `[-1.0, 1.0]`.
    pub correlation: f64,
    pub computed_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
