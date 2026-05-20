//! API Key 存储 — AES-256-GCM 加密存储

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use sea_orm::{entity::prelude::*, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::utils::error::AppError;

pub use crate::db::exchange_api_keys::exchange_api_keys as ek;

// ─── Encryption Helpers ─────────────────────────────────────────────

/// 从环境变量获取 32 字节 master key
pub fn get_master_key() -> Result<[u8; 32], AppError> {
    let hex_key = std::env::var("ENCRYPTION_MASTER_KEY").unwrap_or_else(|_| "0".repeat(64));
    let key_bytes = hex::decode(&hex_key).map_err(|e| {
        error!(error = %e, "Failed to decode ENCRYPTION_MASTER_KEY hex");
        AppError::Internal("Invalid ENCRYPTION_MASTER_KEY".into())
    })?;
    let mut key = [0u8; 32];
    if key_bytes.len() < 32 {
        return Err(AppError::Internal("ENCRYPTION_MASTER_KEY too short".into()));
    }
    key.copy_from_slice(&key_bytes[..32]);
    Ok(key)
}

/// AES-256-GCM 加密 secret，返回 (nonce_b64, combined_b64)
pub fn encrypt_secret(
    master_key: &[u8; 32],
    plaintext: &str,
) -> Result<(String, String), AppError> {
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AppError::Internal(format!("Cipher init error: {}", e)))?;
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| {
        error!(error = %e, "AES-GCM encryption failed");
        AppError::Internal(format!("Encryption failed: {}", e))
    })?;
    let combined = [nonce_bytes.as_slice(), &ciphertext].concat();
    Ok((BASE64.encode(nonce_bytes), BASE64.encode(&combined)))
}

/// AES-256-GCM 解密
pub fn decrypt_secret(
    master_key: &[u8; 32],
    nonce_b64: &str,
    combined_b64: &str,
) -> Result<String, AppError> {
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AppError::Internal(format!("Cipher init error: {}", e)))?;
    let nonce_bytes = BASE64
        .decode(nonce_b64)
        .map_err(|e| AppError::Internal(format!("Invalid nonce base64: {}", e)))?;
    let combined = BASE64
        .decode(combined_b64)
        .map_err(|e| AppError::Internal(format!("Invalid ciphertext base64: {}", e)))?;
    if nonce_bytes.len() != 12 || combined.len() < 12 {
        return Err(AppError::Internal("Invalid encrypted data length".into()));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = &combined[12..];
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Internal("Decryption failed — invalid key or corrupted data".into()))
        .and_then(|plaintext| {
            String::from_utf8(plaintext).map_err(|e| {
                AppError::Internal(format!("Decrypted data is not valid UTF-8: {}", e))
            })
        })
}

/// exchange_api_keys 表行结构
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeApiKey {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub exchange: String,
    pub api_key: String,
    pub secret_encrypted: String,
    pub nonce: String,
    pub permissions: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

/// API Key 存储服务
#[derive(Clone)]
pub struct ApiKeyStore {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
}

impl ApiKeyStore {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self { db, master_key }
    }

    /// Upsert API Key（加密 secret 后存入 DB）
    pub async fn upsert(
        &self,
        user_id: uuid::Uuid,
        exchange: &str,
        api_key_plain: &str,
        secret_plain: &str,
        permissions: &str,
    ) -> Result<ExchangeApiKey, AppError> {
        let (secret_encrypted, nonce) = encrypt_secret(&self.master_key, secret_plain)?;
        let id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        let exch = exchange.to_string();
        let perms = permissions.to_string();

        // 先查找是否已存在
        let existing = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        if let Some(record) = existing {
            let mut active: ek::ActiveModel = record.into();
            active.api_key = sea_orm::Set(api_key_plain.to_string());
            active.secret_encrypted = sea_orm::Set(secret_encrypted.clone());
            active.nonce = sea_orm::Set(nonce.clone());
            active.permissions = sea_orm::Set(permissions.to_string());
            active.is_active = sea_orm::Set(true);
            active.update(self.db.as_ref()).await.map_err(|e| {
                error!(error = %e, "Failed to update API key");
                AppError::Internal(format!("DB error: {}", e))
            })?;
            info!(user_id = %user_id, exchange, "API key updated");
            Ok(ExchangeApiKey {
                id,
                user_id,
                exchange: exch,
                api_key: api_key_plain.to_string(),
                secret_encrypted,
                nonce,
                permissions: perms,
                is_active: true,
                last_used_at: None,
                created_at: now,
            })
        } else {
            let active = ek::ActiveModel {
                id: sea_orm::Set(id),
                user_id: sea_orm::Set(user_id),
                exchange: sea_orm::Set(exchange.to_string()),
                api_key: sea_orm::Set(api_key_plain.to_string()),
                secret_encrypted: sea_orm::Set(secret_encrypted.clone()),
                nonce: sea_orm::Set(nonce.clone()),
                permissions: sea_orm::Set(permissions.to_string()),
                is_active: sea_orm::Set(true),
                last_used_at: sea_orm::Set(None),
                created_at: sea_orm::Set(now),
            };
            active.insert(self.db.as_ref()).await.map_err(|e| {
                error!(error = %e, "Failed to insert API key");
                AppError::Internal(format!("DB error: {}", e))
            })?;
            info!(user_id = %user_id, exchange, "API key saved");
            Ok(ExchangeApiKey {
                id,
                user_id,
                exchange: exch,
                api_key: api_key_plain.to_string(),
                secret_encrypted,
                nonce,
                permissions: perms,
                is_active: true,
                last_used_at: None,
                created_at: now,
            })
        }
    }

    /// 按用户 ID 查询所有启用的 Key（不返回明文 secret）
    pub async fn find_by_user(&self, user_id: uuid::Uuid) -> Result<Vec<ExchangeApiKey>, AppError> {
        let rows = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| ExchangeApiKey {
                id: r.id,
                user_id: r.user_id,
                exchange: r.exchange,
                api_key: r.api_key,
                secret_encrypted: r.secret_encrypted,
                nonce: r.nonce,
                permissions: r.permissions,
                is_active: r.is_active,
                last_used_at: r.last_used_at,
                created_at: r.created_at,
            })
            .collect())
    }

    /// 获取启用的 Key（含明文 secret）
    pub async fn get_active_key(
        &self,
        user_id: uuid::Uuid,
        exchange: &str,
    ) -> Result<Option<(ExchangeApiKey, String)>, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .filter(ek::Column::IsActive.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        match record {
            Some(r) => {
                let secret = decrypt_secret(&self.master_key, &r.nonce, &r.secret_encrypted)?;
                Ok(Some((
                    ExchangeApiKey {
                        id: r.id,
                        user_id: r.user_id,
                        exchange: r.exchange,
                        api_key: r.api_key,
                        secret_encrypted: r.secret_encrypted,
                        nonce: r.nonce,
                        permissions: r.permissions,
                        is_active: r.is_active,
                        last_used_at: r.last_used_at,
                        created_at: r.created_at,
                    },
                    secret,
                )))
            }
            None => Ok(None),
        }
    }

    /// 软删除（禁用）API Key
    pub async fn delete(&self, user_id: uuid::Uuid, exchange: &str) -> Result<(), AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        if let Some(r) = record {
            let mut active: ek::ActiveModel = r.into();
            active.is_active = sea_orm::Set(false);
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        }
        info!(user_id = %user_id, exchange, "API key deactivated");
        Ok(())
    }

    /// 获取加密的 secret（nonce:ciphertext 格式，供签名客户端解密）
    pub async fn get_encrypted_for_decrypt(
        &self,
        user_id: uuid::Uuid,
        exchange: &str,
    ) -> Result<String, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .filter(ek::Column::IsActive.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        match record {
            Some(r) => Ok(format!("{}:{}", r.nonce, r.secret_encrypted)),
            None => Err(AppError::NotFound("API key not found".into())),
        }
    }

    /// 获取明文 API Key
    pub async fn get_api_key_plain(
        &self,
        user_id: uuid::Uuid,
        exchange: &str,
    ) -> Result<String, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .filter(ek::Column::IsActive.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        match record {
            Some(r) => Ok(r.api_key),
            None => Err(AppError::NotFound("API key not found".into())),
        }
    }

    /// 解密 secret（供 SignedBinanceClient 调用）
    pub fn decrypt_secret_for_client(
        &self,
        nonce_b64: &str,
        combined_b64: &str,
    ) -> Result<String, AppError> {
        decrypt_secret(&self.master_key, nonce_b64, combined_b64)
    }

    /// 更新最后使用时间
    pub async fn update_last_used(
        &self,
        user_id: uuid::Uuid,
        exchange: &str,
    ) -> Result<(), AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::UserId.eq(user_id))
            .filter(ek::Column::Exchange.eq(exchange))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        if let Some(r) = record {
            let mut active: ek::ActiveModel = r.into();
            active.last_used_at = sea_orm::Set(Some(chrono::Utc::now()));
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        }
        Ok(())
    }

    /// 按 ID 查询单条 API Key（不验证用户，不返回明文 secret）
    pub async fn find_by_id(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<ExchangeApiKey>, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::Id.eq(id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        Ok(record.map(|r| ExchangeApiKey {
            id: r.id,
            user_id: r.user_id,
            exchange: r.exchange,
            api_key: r.api_key,
            secret_encrypted: r.secret_encrypted,
            nonce: r.nonce,
            permissions: r.permissions,
            is_active: r.is_active,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
        }))
    }

    /// 按 ID 查询 API Key（验证用户归属，不返回明文 secret）
    pub async fn find_by_id_and_user(
        &self,
        id: uuid::Uuid,
        user_id: uuid::Uuid,
    ) -> Result<Option<ExchangeApiKey>, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::Id.eq(id))
            .filter(ek::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        Ok(record.map(|r| ExchangeApiKey {
            id: r.id,
            user_id: r.user_id,
            exchange: r.exchange,
            api_key: r.api_key,
            secret_encrypted: r.secret_encrypted,
            nonce: r.nonce,
            permissions: r.permissions,
            is_active: r.is_active,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
        }))
    }

    /// 按 ID 更新 API Key（permissions / is_active）
    pub async fn update_by_id(
        &self,
        id: uuid::Uuid,
        user_id: uuid::Uuid,
        permissions: Option<&str>,
        is_active: Option<bool>,
    ) -> Result<ExchangeApiKey, AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::Id.eq(id))
            .filter(ek::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
            .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

        let mut active: ek::ActiveModel = record.into();
        if let Some(p) = permissions {
            active.permissions = sea_orm::Set(p.to_string());
        }
        if let Some(a) = is_active {
            active.is_active = sea_orm::Set(a);
        }
        let updated = active.update(self.db.as_ref()).await.map_err(|e| {
            error!(error = %e, "Failed to update API key");
            AppError::Internal(format!("DB error: {}", e))
        })?;

        Ok(ExchangeApiKey {
            id: updated.id,
            user_id: updated.user_id,
            exchange: updated.exchange,
            api_key: updated.api_key,
            secret_encrypted: updated.secret_encrypted,
            nonce: updated.nonce,
            permissions: updated.permissions,
            is_active: updated.is_active,
            last_used_at: updated.last_used_at,
            created_at: updated.created_at,
        })
    }

    /// 按 ID 硬删除 API Key
    pub async fn delete_by_id(
        &self,
        id: uuid::Uuid,
        user_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let record = ek::Entity::find()
            .filter(ek::Column::Id.eq(id))
            .filter(ek::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
            .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

        record.delete(self.db.as_ref()).await.map_err(|e| {
            error!(error = %e, "Failed to delete API key");
            AppError::Internal(format!("DB error: {}", e))
        })?;

        info!(user_id = %user_id, key_id = %id, "API key deleted");
        Ok(())
    }

    /// 管理员分页列出所有 API Key（不返回明文 secret）
    pub async fn list_all_paginated(
        &self,
        page: u64,
        size: u64,
    ) -> Result<(Vec<ExchangeApiKey>, u64), AppError> {
        let total = ek::Entity::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))? as u64;

        let offset = (page.saturating_sub(1)) * size;
        let records = ek::Entity::find()
            .order_by_desc(ek::Column::CreatedAt)
            .offset(offset)
            .limit(size)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;

        let keys = records
            .into_iter()
            .map(|r| ExchangeApiKey {
                id: r.id,
                user_id: r.user_id,
                exchange: r.exchange,
                api_key: r.api_key,
                secret_encrypted: r.secret_encrypted,
                nonce: r.nonce,
                permissions: r.permissions,
                is_active: r.is_active,
                last_used_at: r.last_used_at,
                created_at: r.created_at,
            })
            .collect();

        Ok((keys, total))
    }
}
