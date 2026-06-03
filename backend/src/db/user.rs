use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub role_id: Uuid,
    /// P2-1: Telegram chat_id bound by the user. NULL = not bound → TelegramChannel
    /// skips the user. Non-empty = bound → used as `chat_id` in `sendMessage`.
    /// Telegram chat_id is a 64-bit int; VARCHAR(32) gives ASCII slack.
    pub telegram_chat_id: Option<String>,
    /// P3-B: 2FA TOTP shared secret (base32, RFC 4648). Stored NULL until the
    /// user runs `POST /api/v1/auth/2fa/setup`. Once set, the user must verify
    /// a 6-digit TOTP code to enable 2FA (`totp_enabled = true`).
    /// We keep this field even when 2FA is disabled so a user re-enabling
    /// doesn't have to re-scan the QR.
    pub totp_secret: Option<String>,
    /// P3-B: whether 2FA is enforced for login. Setup creates the secret but
    /// only flips this flag on after the user proves possession by
    /// submitting a valid TOTP code.
    pub totp_enabled: bool,
    /// P3-B: hashed backup codes (10 one-time 8-hex-digit recovery codes).
    /// Each code is bcrypt-hashed so a DB leak doesn't grant account access.
    /// We store the array as a JSON column (`Vec<String>` of bcrypt hashes).
    /// NULL = user has never enabled 2FA, or has used all codes.
    /// The plaintext codes are returned ONCE on setup; the client must show
    /// them to the user immediately, then discard them.
    pub backup_codes: Option<Json>,
    pub last_login_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::role::Entity",
        from = "Column::RoleId",
        to = "super::role::Column::Id"
    )]
    Role,
    #[sea_orm(has_many = "super::user_session::Entity")]
    UserSessions,
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl Related<super::user_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserSessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
