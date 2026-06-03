use crate::db::audit_log::{actions, target_types};
use crate::db::{role, user};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    AdminUpdateUserRequest, ChangePasswordRequest, PaginatedResponse, PaginationParams,
    UpdateUserRequest, UserMeResponse, UserResponse, UserUpdateResponse,
};
use crate::services::audit_log;
use crate::services::auth;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

/// Require the authenticated user to have admin role.
///
/// Currently unused — admin authorization is enforced at the router layer via
/// `require_admin_middleware` on the admin sub-router. Kept here for handlers
/// that need to perform an in-handler role check (e.g. self-service escalation).
#[allow(dead_code)]
fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Admin privileges required".into()));
    }
    Ok(())
}

/// Helper to build UserResponse from user model + role
fn build_user_response(user_model: &user::Model, role_model: &role::Model) -> UserResponse {
    UserResponse {
        id: user_model.id,
        username: user_model.username.clone(),
        email: user_model.email.clone(),
        display_name: user_model.display_name.clone(),
        avatar_url: user_model.avatar_url.clone(),
        is_active: user_model.is_active,
        role: crate::models::schemas::RoleResponse {
            id: role_model.id,
            name: role_model.name.clone(),
            display_name: role_model.display_name.clone(),
            description: role_model.description.clone(),
            is_system: role_model.is_system,
            created_at: role_model.created_at,
        },
        last_login_at: user_model.last_login_at,
        created_at: user_model.created_at,
    }
}

/// Internal get_me that returns UserResponse directly (for reuse by auth handler)
pub async fn get_me_internal(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<UserResponse, AppError> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let role_model = role::Entity::find_by_id(user_model.role_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    Ok(build_user_response(&user_model, &role_model))
}

/// List all users (admin only)
pub async fn list_users(
    _user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<crate::models::schemas::UserListResponse>>, AppError> {
    let page = params.page();
    let size = params.size();
    let offset = params.offset();

    let total = user::Entity::find().count(&*db).await? as u64;

    let users = user::Entity::find()
        .order_by_desc(user::Column::CreatedAt)
        .offset(offset)
        .limit(size)
        .all(&*db)
        .await?;

    let mut user_list = Vec::new();
    for u in users {
        let role = role::Entity::find_by_id(u.role_id)
            .one(&*db)
            .await?
            .ok_or_else(|| AppError::Internal("Role not found".into()))?;

        user_list.push(build_user_response(&u, &role));
    }

    Ok(Json(ApiResponse::success(
        crate::models::schemas::UserListResponse(PaginatedResponse {
            items: user_list,
            total,
            page,
            size,
        }),
    )))
}

/// Get current user profile
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    operation_id = "users_get_me",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user profile", body = UserMeResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_me(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<UserMeResponse>>, AppError> {
    let resp = get_me_internal(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(UserMeResponse(resp))))
}

/// Update current user profile
pub async fn update_me(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    headers: HeaderMap,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserUpdateResponse>>, AppError> {
    let user_model = user::Entity::find_by_id(user.user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // P3-4: 拿 before 状态供 diff
    let before_snapshot = serde_json::json!({
        "display_name": user_model.display_name,
        "email": user_model.email,
        "avatar_url": user_model.avatar_url,
    });

    let mut active: user::ActiveModel = user_model.clone().into();
    active.updated_at = sea_orm::Set(chrono::Utc::now());

    if let Some(display_name) = body.display_name.clone() {
        #[allow(clippy::collapsible_if)]
        if !display_name.is_empty() {
            active.display_name = sea_orm::Set(Some(display_name));
        }
    }
    if let Some(ref email) = body.email
        && !email.is_empty()
    {
        let existing = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::Id.ne(user.user_id))
            .one(&*db)
            .await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Email already in use".into()));
        }
        active.email = sea_orm::Set(email.clone());
    }
    if let Some(avatar_url) = body.avatar_url.clone() {
        active.avatar_url = sea_orm::Set(Some(avatar_url));
    }

    let updated = active.update(&*db).await?;
    let role = role::Entity::find_by_id(updated.role_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    // P3-4: 写审计 — 个人 profile 变更（含 email）也是敏感操作。
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let after_snapshot = serde_json::json!({
        "display_name": updated.display_name,
        "email": updated.email,
        "avatar_url": updated.avatar_url,
    });
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::USER_UPDATED.to_string(),
        target_type: target_types::USER.to_string(),
        target_id: user.user_id.to_string(),
        before: Some(before_snapshot),
        after: Some(after_snapshot),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&db, event).await;

    Ok(Json(ApiResponse::success(UserUpdateResponse(
        build_user_response(&updated, &role),
    ))))
}

/// Change current user password
#[utoipa::path(
    post,
    path = "/api/v1/users/me/password",
    tag = "users",
    operation_id = "users_change_password",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed", body = crate::models::schemas::ChangePasswordResponse),
        (status = 400, description = "Validation failed (e.g. old password wrong, new password too short)"),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn change_password(
    user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    headers: HeaderMap,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<crate::models::schemas::ChangePasswordResponse>>, AppError> {
    let user_model = user::Entity::find_by_id(user.user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // Verify old password
    let valid = auth::verify_password(&body.old_password, &user_model.password_hash)?;
    if !valid {
        return Err(AppError::BadRequest("Current password is incorrect".into()));
    }

    if body.new_password.len() < 8 {
        return Err(AppError::Validation(
            "New password must be at least 8 characters".into(),
        ));
    }

    let new_hash = auth::hash_password(&body.new_password)?;

    let mut active: user::ActiveModel = user_model.into();
    active.password_hash = sea_orm::Set(new_hash);
    active.updated_at = sea_orm::Set(chrono::Utc::now());
    active.update(&*db).await?;

    // P3-4: 写审计 —— 密码变更不可逆事件，必须留痕。
    // 仅记录 user_id + 事件类型 + 长度是否变更；不写明文/哈希。
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::AUTH_PASSWORD_CHANGED.to_string(),
        target_type: target_types::AUTH.to_string(),
        target_id: user.user_id.to_string(),
        before: None,
        after: Some(serde_json::json!({
            "event": "password.changed",
            "self_service": true,
        })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&db, event).await;

    Ok(Json(ApiResponse::success(
        crate::models::schemas::ChangePasswordResponse {
            message: "Password changed successfully".into(),
        },
    )))
}

/// Admin: update user (including role)
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}",
    tag = "users",
    operation_id = "users_admin_update",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Target user id"),
    ),
    request_body = AdminUpdateUserRequest,
    responses(
        (status = 200, description = "Updated user", body = UserResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn admin_update_user(
    _user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<AdminUpdateUserRequest>,
) -> Result<Json<ApiResponse<crate::models::schemas::UserResponse>>, AppError> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // P3-4: 拿 before 状态（role_id, is_active）供 diff
    let before_role_id = user_model.role_id;
    let before_is_active = user_model.is_active;

    let mut active: user::ActiveModel = user_model.clone().into();
    active.updated_at = sea_orm::Set(chrono::Utc::now());

    if let Some(ref email) = body.email
        && !email.is_empty()
    {
        let existing = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::Id.ne(user_id))
            .one(&*db)
            .await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Email already in use".into()));
        }
        active.email = sea_orm::Set(email.clone());
    }
    if let Some(avatar_url) = body.avatar_url {
        active.avatar_url = sea_orm::Set(Some(avatar_url));
    }
    if let Some(is_active) = body.is_active {
        active.is_active = sea_orm::Set(is_active);
    }
    if let Some(role_id) = body.role_id {
        // Verify role exists
        let role = role::Entity::find_by_id(role_id).one(&*db).await?;
        if role.is_none() {
            return Err(AppError::NotFound("Role not found".into()));
        }
        active.role_id = sea_orm::Set(role_id);
    }

    let updated = active.update(&*db).await?;
    let role = role::Entity::find_by_id(updated.role_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::Internal("Role not found".into()))?;

    // P3-4: role_id / is_active 变更都要写审计。
    // 优先用 USER_ROLE_CHANGED（语义最强）；纯 is_active 切换用 USER_DEACTIVATED
    // / USER_UPDATED 标记。注意 body.role_id / body.is_active 是 Option
    // —— 只有实际改变时才落审计。
    if let Some(new_role) = body.role_id {
        if new_role != before_role_id {
            let (ip, ua, req_id) =
                audit_log::extract_audit_context(&headers, "0.0.0.0");
            let event = audit_log::AuditEvent {
                user_id: Some(user_id),
                action: actions::USER_ROLE_CHANGED.to_string(),
                target_type: target_types::USER.to_string(),
                target_id: user_id.to_string(),
                before: Some(serde_json::json!({
                    "role_id": before_role_id,
                })),
                after: Some(serde_json::json!({
                    "role_id": new_role,
                })),
                ip_address: ip,
                user_agent: ua,
                request_id: req_id,
            };
            audit_log::record(&db, event).await;
        }
    }
    if let Some(new_active) = body.is_active {
        if new_active != before_is_active {
            let (ip, ua, req_id) =
                audit_log::extract_audit_context(&headers, "0.0.0.0");
            let action = if new_active {
                actions::USER_UPDATED.to_string()
            } else {
                actions::USER_DEACTIVATED.to_string()
            };
            let event = audit_log::AuditEvent {
                user_id: Some(user_id),
                action,
                target_type: target_types::USER.to_string(),
                target_id: user_id.to_string(),
                before: Some(serde_json::json!({ "is_active": before_is_active })),
                after: Some(serde_json::json!({ "is_active": new_active })),
                ip_address: ip,
                user_agent: ua,
                request_id: req_id,
            };
            audit_log::record(&db, event).await;
        }
    }

    Ok(Json(ApiResponse::success(build_user_response(
        &updated, &role,
    ))))
}

/// Admin: delete user
pub async fn admin_delete_user(
    _user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<crate::models::schemas::ChangePasswordResponse>>, AppError> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // Delete sessions first, then the user
    use sea_orm::EntityTrait as _;
    crate::db::user_session::Entity::delete_many()
        .filter(crate::db::user_session::Column::UserId.eq(user_id))
        .exec(&*db)
        .await?;

    user_model.delete(&*db).await?;

    // P3-4: 删除账户是高危操作，必须留痕
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user_id),
        action: actions::USER_DELETED.to_string(),
        target_type: target_types::USER.to_string(),
        target_id: user_id.to_string(),
        before: None,
        after: Some(serde_json::json!({ "deleted_id": user_id })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&db, event).await;

    Ok(Json(ApiResponse::success(
        crate::models::schemas::ChangePasswordResponse {
            message: "User deleted successfully".into(),
        },
    )))
}

/// List all roles
pub async fn list_roles(
    _user: AuthenticatedUser,
    State(db): State<std::sync::Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<crate::models::schemas::RoleListResponse>>, AppError> {
    let roles = role::Entity::find()
        .order_by_asc(role::Column::Name)
        .all(&*db)
        .await?;

    let role_list: Vec<crate::models::schemas::RoleResponse> = roles
        .into_iter()
        .map(|r| crate::models::schemas::RoleResponse {
            id: r.id,
            name: r.name,
            display_name: r.display_name,
            description: r.description,
            is_system: r.is_system,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(
        crate::models::schemas::RoleListResponse(role_list),
    )))
}
