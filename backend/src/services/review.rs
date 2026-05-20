//! Review service — strategy review workflow (P2-F2)
//!
//! Workflow: pending_review → approved | rejected
//! Review state is stored in a separate `strategy_reviews` table (not on Strategy model)
//! to avoid DeriveEntityModel compile-time introspection issues.

use crate::db::review as review_module;
use crate::db::review::{
    ActiveModel as ReviewActiveModel, Column as ReviewColumn, Entity as ReviewEntity,
    Model as ReviewModel, ReviewStatus,
};
use crate::db::strategy::ActiveModel as StrategyActiveModel;
use crate::db::strategy::Entity as StrategyEntity;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};
use uuid::Uuid;

/// Submit a strategy for review (user action)
pub async fn submit_for_review(
    db: &DatabaseConnection,
    strategy_id: Uuid,
    user_id: Uuid,
) -> Result<ReviewModel, AppError> {
    // Verify strategy exists and belongs to user
    let strategy = StrategyEntity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;

    if strategy.user_id != user_id {
        return Err(AppError::Forbidden(
            "You can only submit your own strategies for review".into(),
        ));
    }

    let now = chrono::Utc::now();

    // Upsert review record
    let review = ReviewActiveModel {
        id: Set(Uuid::new_v4()),
        strategy_id: Set(strategy_id),
        review_status: Set(ReviewStatus::PendingReview.as_str().to_string()),
        rejection_reason: Set(None),
        submitted_at: Set(Some(now)),
        reviewed_at: Set(None),
        reviewed_by: Set(None),
    };

    let saved = review.insert(db).await?;

    // Update strategy status to indicate submitted
    let mut strategy_model: StrategyActiveModel = strategy.into();
    strategy_model.status = Set("pending_review".to_string());
    strategy_model.update(db).await?;

    Ok(saved)
}

/// Approve a strategy (reviewer action)
pub async fn approve_strategy(
    db: &DatabaseConnection,
    strategy_id: Uuid,
    reviewer_id: Uuid,
) -> Result<ReviewModel, AppError> {
    let review = ReviewEntity::find()
        .filter(ReviewColumn::StrategyId.eq(strategy_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Review not found for this strategy".into()))?;

    if review.review_status != ReviewStatus::PendingReview.as_str() {
        return Err(AppError::Validation(format!(
            "Strategy is already in status: {}",
            review.review_status
        )));
    }

    let now = chrono::Utc::now();

    let mut review_model: ReviewActiveModel = review.into();
    review_model.review_status = Set(ReviewStatus::Approved.as_str().to_string());
    review_model.reviewed_at = Set(Some(now));
    review_model.reviewed_by = Set(Some(reviewer_id));
    let updated = review_model.update(db).await?;

    // Update strategy status
    let strategy = StrategyEntity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;
    let mut strategy_model: StrategyActiveModel = strategy.into();
    strategy_model.status = Set("approved".to_string());
    strategy_model.update(db).await?;

    Ok(updated)
}

/// Reject a strategy (reviewer action)
pub async fn reject_strategy(
    db: &DatabaseConnection,
    strategy_id: Uuid,
    reviewer_id: Uuid,
    reason: Option<String>,
) -> Result<ReviewModel, AppError> {
    let review = ReviewEntity::find()
        .filter(ReviewColumn::StrategyId.eq(strategy_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Review not found for this strategy".into()))?;

    if review.review_status != ReviewStatus::PendingReview.as_str() {
        return Err(AppError::Validation(format!(
            "Strategy is already in status: {}",
            review.review_status
        )));
    }

    let now = chrono::Utc::now();

    let mut review_model: ReviewActiveModel = review.into();
    review_model.review_status = Set(ReviewStatus::Rejected.as_str().to_string());
    review_model.rejection_reason = Set(reason);
    review_model.reviewed_at = Set(Some(now));
    review_model.reviewed_by = Set(Some(reviewer_id));
    let updated = review_model.update(db).await?;

    // Update strategy status
    let strategy = StrategyEntity::find_by_id(strategy_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Strategy not found".into()))?;
    let mut strategy_model: StrategyActiveModel = strategy.into();
    strategy_model.status = Set("rejected".to_string());
    strategy_model.update(db).await?;

    Ok(updated)
}

/// List all pending reviews (admin view)
pub async fn list_pending_reviews(db: &DatabaseConnection) -> Result<Vec<ReviewModel>, AppError> {
    let reviews = ReviewEntity::find()
        .filter(ReviewColumn::ReviewStatus.eq(ReviewStatus::PendingReview.as_str()))
        .all(db)
        .await?;
    Ok(reviews)
}

/// Get review record for a specific strategy
pub async fn get_strategy_review(
    db: &DatabaseConnection,
    strategy_id: Uuid,
) -> Result<Option<ReviewModel>, AppError> {
    let review = ReviewEntity::find()
        .filter(ReviewColumn::StrategyId.eq(strategy_id))
        .one(db)
        .await?;
    Ok(review)
}
