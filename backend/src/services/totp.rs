//! services/totp.rs — P3-B RFC 6238 TOTP (Time-based One-Time Password) service
//!
//! 中文：实现 2FA (TOTP) 的核心逻辑，包括：
//!   1. 生成 base32 共享密钥 (`generate_secret`)
//!   2. 验证 6 位 TOTP 码 (`verify_code`，默认允许 ±1 时间步的时钟漂移)
//!   3. 构造 `otpauth://` URI，Google Authenticator / Authy / 1Password 都能扫
//!   4. 把 URI 渲染成 PNG (data: URL) 便于前端直接展示
//!   5. 生成 10 个一次性 backup code
//!
//! English: Implements the 2FA (TOTP) core for P3-B:
//!   1. Generate a base32 shared secret
//!   2. Verify a 6-digit TOTP code (±1 step skew for clock drift)
//!   3. Build an `otpauth://` URI scannable by Google Authenticator / Authy / 1Password
//!   4. Render the URI to a PNG data URL
//!   5. Generate 10 one-time backup recovery codes
//!
//! Design notes:
//!   - We use `Algorithm::SHA1` (the historical default — still what 99% of
//!     authenticator apps default to and the only one RFC 6238 mandates).
//!     `SHA256` / `SHA512` are out of scope; adding them later is a one-liner.
//!   - Step is 30s (RFC 6238 §4.1 recommended). Digits is 6 (industry default).
//!   - The `Secret` is 160 bits (20 bytes) — well above the 128-bit floor
//!     Google Authenticator enforces.
//!   - QR encoding uses the `qrcode` crate (0.14) with the `image` feature so
//!     we can return a `data:image/png;base64,…` string the `<img src=…>`
//!     can render without extra plumbing on the front-end.
//!   - Backup codes are 8 hex-digit (32 bits of entropy) which is the
//!     Google Authenticator convention. They're bcrypt-hashed before storage
//!     in `handlers::totp::verify_setup`.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use qrcode::QrCode;
use rand::Rng;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::utils::error::AppError;

/// TOTP step (seconds). RFC 6238 §4.1 recommends 30s. Authenticator apps
/// all default to 30s, so we use the same.
pub const TOTP_STEP_SECS: u64 = 30;

/// Number of digits in the TOTP code. 6 is the industry default (Google
/// Authenticator, Authy, 1Password all use 6).
pub const TOTP_DIGITS: usize = 6;

/// Number of backup codes generated on 2FA setup.
pub const BACKUP_CODE_COUNT: usize = 10;

/// Length of each backup code in hex characters (8 hex = 32 bits entropy).
pub const BACKUP_CODE_HEX_LEN: usize = 8;

/// Symmetric clock-skew tolerance: a code valid in the immediately previous
/// or next 30s window is also accepted. This is what every major TOTP
/// implementation does — it makes the user experience bearable when the
/// phone clock drifts by a few seconds.
pub const TOTP_SKEW: u8 = 1;

/// Issuer (app name) embedded in the otpauth URI. Google Authenticator
/// shows this as the account label.
pub const TOTP_ISSUER: &str = "Quant Trading";

/// Helper: build a TOTP instance for verification. The 7-arg constructor
/// in totp-rs 5.x requires the issuer + account_name to be on the struct
/// (used by `get_url`). For verification only we pass empty strings.
fn build_totp(secret_b32: &str, account_name: &str) -> Result<TOTP, AppError> {
    let secret = Secret::Encoded(secret_b32.to_string());
    let secret_bytes = secret.to_bytes().map_err(|e| {
        AppError::Validation(format!("invalid TOTP secret: {}", e))
    })?;
    // `new` is the validating constructor. issuer cannot contain `:` and
    // account_name cannot contain `:` either (RFC 3986 reserved in URI label).
    TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_STEP_SECS,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        account_name.to_string(),
    )
    .map_err(|e| AppError::Validation(format!("invalid TOTP secret: {}", e)))
}

/// Generate a fresh base32 TOTP secret.
///
/// Returns a 160-bit secret encoded in base32 (RFC 4648). The encoded form
/// is what the `otpauth://` URI expects and what Authenticator apps
/// display under "Manual entry" / "Type instead of scanning".
pub fn generate_secret() -> String {
    // 20 bytes = 160 bits, matching SHA-1 block size and well above the
    // 128-bit floor recommended by RFC 4226 §4 (RADIUS).
    let secret = Secret::generate_secret();
    secret.to_encoded().to_string()
}

/// Verify a 6-digit TOTP code against a stored base32 secret.
///
/// `code` should be exactly 6 ASCII digits (we trim whitespace and reject
/// anything else before doing the cryptographic compare).
///
/// Returns `Ok(true)` if the code is valid, `Ok(false)` if it isn't. The
/// only error path is a malformed `code` string (non-digit, wrong length),
/// which surfaces as `AppError::Validation`.
pub fn verify_code(secret_b32: &str, code: &str) -> Result<bool, AppError> {
    let code = code.trim();
    if code.len() != TOTP_DIGITS || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Validation(format!(
            "TOTP code must be exactly {} digits",
            TOTP_DIGITS
        )));
    }

    let totp = build_totp(secret_b32, "")?;
    // `check` already applies the ±TOTP_SKEW tolerance. Returns true on
    // match, false otherwise; never panics.
    Ok(totp.check(
        code,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    ))
}

/// Build the `otpauth://totp/...` URI that Authenticator apps consume.
///
/// `issuer` is your app name (e.g. "Quant Trading"). It's also embedded
/// in the URI so apps display it as the account label, and the Google
/// Authenticator "manual entry" path shows `Issuer: <name>`.
pub fn generate_otpauth_uri(
    user_email: &str,
    secret_b32: &str,
    _issuer: &str,
) -> Result<String, AppError> {
    let totp = build_totp(secret_b32, user_email)?;
    // `get_url` returns the canonical `otpauth://totp/<issuer>:<account>?...`
    // form, with the issuer and account_name URL-encoded as needed.
    Ok(totp.get_url())
}

/// Encode an `otpauth://` URI as a PNG data URL (`data:image/png;base64,…`).
///
/// The image is 256×256 — big enough to scan reliably from a desktop screen
/// at the typical LoginView 2FA card size, small enough to embed in JSON
/// without bloating the response (≈ 4–6 KB after base64).
pub fn generate_qr_data_url(otpauth_uri: &str) -> Result<String, AppError> {
    // `QrCode::new` validates the input is short enough to encode; for our
    // ~80-char otpauth URI this always succeeds.
    let code = QrCode::new(otpauth_uri.as_bytes())
        .map_err(|e| AppError::Internal(format!("QR encode failed: {}", e)))?;

    // `render::<image::Luma<_>>` produces a `GrayImage` (8-bit grayscale).
    // We resize to 256×256 with nearest-neighbour sampling (the source is
    // 1-bit per module anyway, so smoother filters would only soften it).
    let image = code
        .render::<image::Luma<u8>>()
        .min_dimensions(256, 256)
        .build();

    // Encode PNG → base64 → data URL.
    let mut png_bytes: Vec<u8> = Vec::with_capacity(8 * 1024);
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::L8,
            )
            .map_err(|e| AppError::Internal(format!("PNG encode failed: {}", e)))?;
    }

    let b64 = BASE64.encode(&png_bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

/// Generate `BACKUP_CODE_COUNT` one-time recovery codes.
///
/// Each code is 8 hex digits (`BACKUP_CODE_HEX_LEN`). Hex was chosen over
/// the [a-z0-9] format Google uses because uppercase hex is unambiguous
/// when typed on a mobile keyboard (no I/l/O/0 confusion). The codes are
/// returned in **plaintext** and must be shown to the user once; the
/// caller (`handlers::totp::verify_setup`) hashes them via bcrypt before
/// persisting.
///
/// The codes are returned in fixed order to make the UI "first 10 lines"
/// presentation easy. Uniqueness is verified by the caller (we could
/// also loop here, but `BACKUP_CODE_COUNT=10` from a 32-bit space has
/// collision probability ≈ 4e-8, well below user-error rate).
pub fn generate_backup_codes() -> Vec<String> {
    let mut rng = rand::thread_rng();
    (0..BACKUP_CODE_COUNT)
        .map(|_| {
            // 8 hex chars = 4 random bytes = 32 bits of entropy per code.
            // `gen::<[u8; 4]>()` is the rand 0.8 turbofish syntax for
            // sized array generation.
            let bytes: [u8; 4] = rng.r#gen();
            hex::encode(bytes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Compute the TOTP code for a given (now-bounded) time. Used to
    /// generate the expected code in tests so we don't need an authenticator
    /// app in the loop. The implementation mirrors `verify_code`'s settings.
    fn code_at(secret_b32: &str, t_secs: u64) -> String {
        let totp = build_totp(secret_b32, "test@example.com").unwrap();
        totp.generate(t_secs)
    }

    #[test]
    fn generate_secret_is_unique_and_base32() {
        let s1 = generate_secret();
        let s2 = generate_secret();
        assert_ne!(s1, s2, "two calls should yield different secrets");
        // base32 alphabet is A-Z and 2-7. Strip padding `=` and verify
        // every char is in the alphabet.
        let stripped: String = s1.replace('=', "");
        for c in stripped.chars() {
            assert!(
                c.is_ascii_alphanumeric() && !"0189".contains(c),
                "non-base32 char in secret: {}",
                c
            );
        }
    }

    #[test]
    fn verify_code_accepts_current_code() {
        let secret = generate_secret();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code = code_at(&secret, now);
        assert!(verify_code(&secret, &code).unwrap());
    }

    #[test]
    fn verify_code_rejects_wrong_code() {
        let secret = generate_secret();
        // Build a 6-digit code that's guaranteed not to match by XOR-ing
        // each digit with 1 (mod 10), so 123456 → 234567.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code = code_at(&secret, now);
        let bad: String = code
            .chars()
            .map(|c| {
                let d = c.to_digit(10).unwrap();
                std::char::from_digit((d + 1) % 10, 10).unwrap()
            })
            .collect();
        assert!(
            !verify_code(&secret, &bad).unwrap(),
            "expected bad code {} to be rejected, but verify_code returned true",
            bad
        );
    }

    #[test]
    fn verify_code_tolerates_clock_skew_one_step() {
        // TOTP_SKEW = 1, so a code from the *previous* 30s window should
        // still verify. The ±1 step tolerance is what makes the UX
        // bearable when the phone clock drifts.
        let secret = generate_secret();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let one_step_ago = now - TOTP_STEP_SECS;
        let old_code = code_at(&secret, one_step_ago);
        assert!(
            verify_code(&secret, &old_code).unwrap(),
            "code from previous window should still verify"
        );
    }

    #[test]
    fn verify_code_rejects_two_steps_old() {
        // Two steps away exceeds the ±1 tolerance, so we expect rejection.
        let secret = generate_secret();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_steps_ago = now - 2 * TOTP_STEP_SECS;
        let old_code = code_at(&secret, two_steps_ago);
        assert!(
            !verify_code(&secret, &old_code).unwrap(),
            "code from two windows ago should be rejected"
        );
    }

    #[test]
    fn verify_code_rejects_malformed_input() {
        let secret = generate_secret();
        // Too short
        assert!(verify_code(&secret, "12345").is_err());
        // Too long
        assert!(verify_code(&secret, "1234567").is_err());
        // Non-digit
        assert!(verify_code(&secret, "abcdef").is_err());
        // Mixed
        assert!(verify_code(&secret, "12345a").is_err());
        // Empty / whitespace
        assert!(verify_code(&secret, "").is_err());
        assert!(verify_code(&secret, "       ").is_err());
    }

    #[test]
    fn generate_otpauth_uri_has_correct_shape() {
        let secret = generate_secret();
        let uri = generate_otpauth_uri("alice@example.com", &secret, "Quant Trading").unwrap();
        // Standard otpauth URI shape: otpauth://totp/<issuer>:<account>?...
        assert!(uri.starts_with("otpauth://totp/"), "got: {}", uri);
        // `urlencoding::encode` encodes space as `+` in application/x-www-form
        // style, but for URI path segments it may use `%20`. Accept either.
        assert!(
            uri.contains("Quant") && (uri.contains("Trading") || uri.contains("Trading%20") || uri.contains("Trading+")),
            "issuer not encoded: {}",
            uri
        );
        assert!(uri.contains("alice") && uri.contains("example.com"));
        assert!(uri.contains("secret="));
    }

    #[test]
    fn generate_qr_data_url_is_a_png_data_url() {
        let secret = generate_secret();
        let uri = generate_otpauth_uri("a@b.com", &secret, "Issuer").unwrap();
        let data_url = generate_qr_data_url(&uri).unwrap();
        assert!(data_url.starts_with("data:image/png;base64,"));
        // base64 of a valid PNG starts with `iVBORw0KGgo` (the PNG magic
        // 89 50 4E 47 0D 0A 1A 0A in base64).
        let b64 = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let png_bytes = BASE64.decode(b64).unwrap();
        assert_eq!(&png_bytes[0..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn generate_backup_codes_count_and_shape() {
        let codes = generate_backup_codes();
        assert_eq!(codes.len(), BACKUP_CODE_COUNT);
        for c in &codes {
            assert_eq!(c.len(), BACKUP_CODE_HEX_LEN, "code {} wrong length", c);
            assert!(c.chars().all(|x| x.is_ascii_hexdigit()), "non-hex: {}", c);
        }
    }

    #[test]
    fn generate_backup_codes_collision_rare() {
        // Two consecutive calls should not collide (probabilistic check).
        // With 32 bits of entropy per code and 10 codes per call, the
        // birthday-paradox collision probability across 20 codes is ≈ 4e-8.
        let a = generate_backup_codes();
        let b = generate_backup_codes();
        let set_a: std::collections::HashSet<_> = a.iter().collect();
        for code in &b {
            assert!(!set_a.contains(code), "collision between runs: {}", code);
        }
    }
}
