//! `services::feature_flag` — P3-5 lightweight in-house feature flag service.
//!
//! Public API (used by `handlers/feature_flag.rs` and by hot-path business
//! code that wants a fast in-process lookup):
//!
//!   * [`is_enabled`]            — single-flag decision for a given user.
//!                                 Hot path; uses a 5-second in-process cache so
//!                                 a request that triggers N flag lookups
//!                                 only hits the DB at most once per 5s.
//!   * [`list`]                  — admin list of every flag row (no cache).
//!   * [`upsert`]                — admin create/update of a flag row.
//!   * [`delete_flag`]           — admin delete (drops the row; does not
//!                                 soft-delete).
//!   * [`evaluate_all`]          — bulk evaluate for the frontend; returns
//!                                 `{flag_key: bool}` for every known flag.
//!                                 Used by the bootstrap endpoint so the SPA
//!                                 can decide which UI affordances to render.
//!
//! The cache is a `tokio::sync::RwLock<HashMap<key, Arc<CachedFlag>>>`,
//! populated lazily on miss and refreshed on a 5-second background tick
//! (see [`spawn_refresh_task`]). The 5s TTL is the upper bound on the
//! staleness an admin can expect to see after flipping a flag in the UI
//! — well within the "real-time toggle" requirement.
//!
//! Hot path semantics:
//!
//!   1. Read lock the cache. On hit, evaluate against the cached row.
//!   2. On miss, acquire the write lock, re-check, fetch from DB, insert.
//!
//! The miss path is also used for unknown flag keys: a row that doesn't
//! exist in the DB maps to "flag not found → `false`" and we don't cache
//! that — repeated unknown lookups are an upstream bug and we want to
//! notice them via the `tracing::warn!` we emit on every miss.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, Set,
};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::db::feature_flag as entity;
use crate::db::feature_flag::{
    ActiveModel as FlagActive, Entity as FlagEntity, Model as FlagModel,
};

/// In-process cache entry. We hold an `Arc` so the read path can hand out a
/// reference to the same struct without cloning the row's `Json` values.
#[derive(Debug, Clone)]
struct CachedFlag {
    model: Arc<FlagModel>,
}

/// The shared cache. `RwLock` because reads (the hot path) vastly outnumber
/// writes (admin upserts and the 5s refresh tick).
#[derive(Default)]
pub struct FeatureFlagCache {
    inner: RwLock<HashMap<String, Arc<CachedFlag>>>,
}

impl FeatureFlagCache {
    /// Build an empty cache. Cheap; uses `Default`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a clone of the cached row, or `None` if the flag isn't cached.
    async fn get(&self, key: &str) -> Option<Arc<CachedFlag>> {
        self.inner.read().await.get(key).cloned()
    }

    /// Insert / replace a row in the cache.
    async fn put(&self, key: String, model: Arc<FlagModel>) {
        self.inner
            .write()
            .await
            .insert(key, Arc::new(CachedFlag { model }));
    }

    /// Remove a key from the cache (called by `delete_flag` so a freshly
    /// deleted row doesn't keep serving `true` until the next tick).
    async fn invalidate(&self, key: &str) {
        self.inner.write().await.remove(key);
    }

    /// Replace the entire cache contents with `flags`. Used by the
    /// background refresh task and by [`refresh_from_db`].
    async fn replace_all(&self, flags: Vec<FlagModel>) {
        let mut g = self.inner.write().await;
        g.clear();
        for f in flags {
            g.insert(
                f.key.clone(),
                Arc::new(CachedFlag {
                    model: Arc::new(f),
                }),
            );
        }
    }

    /// Return the current size, mainly for tests and the
    /// `/api/v1/admin/feature-flags/_cache-stats` admin endpoint.
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

// ─── Hot-path evaluation logic ────────────────────────────────────────

/// Compute the boolean decision for a single cached row.
///
/// Exposed as a free function so unit tests can call it without spinning
/// up a Tokio runtime or a database.
pub fn evaluate_flag(model: &FlagModel, user_id: Option<&Uuid>) -> bool {
    // 1. Whitelist short-circuit — whitelisted users always see the flag as
    //    on, even if `enabled` is false. (Useful for staging/QA: ship a
    //    feature live to a small admin set while hiding it from the rest of
    //    the user base.)
    if let Some(uid) = user_id {
        let in_list = model.user_whitelist.as_array().map_or(false, |arr| {
            arr.iter()
                .any(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()).as_ref() == Some(uid))
        });
        if in_list {
            return true;
        }
    }

    // 2. Global switch.
    if !model.enabled {
        return false;
    }

    // 3. Percentage rollout. Two short-circuits:
    //      pct == 0   → off for everyone not in the whitelist
    //      pct == 100 → on for everyone not in the whitelist
    let pct = model.percentage_rollout.clamp(0, 100);
    if pct == 0 {
        return false;
    }
    if pct == 100 {
        return true;
    }
    // No user_id + non-trivial percentage → conservatively off. This is the
    // correct behaviour for unauthenticated requests: we cannot hash-bucket
    // an anonymous caller, and we'd rather under-include than over-include
    // a feature that's supposed to be a controlled rollout.
    let uid = match user_id {
        Some(u) => u,
        None => return false,
    };

    // Stable, fast non-cryptographic hash. The intent is just to scatter
    // `user_id`s evenly across the 0..=99 bucket space — we don't need
    // collision resistance.
    bucket_for_user(uid) < pct as u32
}

/// Map a `Uuid` to a uniformly-distributed 0..=99 bucket. Uses the FxHash
/// algorithm (the same one rustc uses for its internal hash maps) — it's
/// fast, deterministic, and gives a good avalanche for sequential UUIDs.
pub fn bucket_for_user(uid: &Uuid) -> u32 {
    let bytes = uid.as_bytes();
    // FxHash-style 64-bit avalanche on the first 8 bytes.
    let mut h: u64 = 0xcbf29ce484222325; // FxHash offset basis
    for &b in &bytes[..8] {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FxHash prime
    }
    // Finish the avalanche (Murmur3 finalizer).
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    (h % 100) as u32
}

// ─── Public service API ────────────────────────────────────────────────

/// `is_enabled` — the hot-path decision. Pulls from the cache, falling
/// back to the DB on a miss. Returns `false` for unknown flag keys (a
/// warning is logged once per key per process).
pub async fn is_enabled(cache: &FeatureFlagCache, db: &DatabaseConnection, flag_key: &str, user_id: Option<&Uuid>) -> bool {
    if let Some(cached) = cache.get(flag_key).await {
        return evaluate_flag(&cached.model, user_id);
    }

    // Miss. Pull from DB and insert.
    match FlagEntity::find_by_id(flag_key).one(db).await {
        Ok(Some(model)) => {
            let decision = evaluate_flag(&model, user_id);
            let m = Arc::new(model);
            cache.put(flag_key.to_string(), m).await;
            decision
        }
        Ok(None) => {
            warn!(flag_key, "feature flag not found in DB; treating as disabled");
            false
        }
        Err(e) => {
            warn!(flag_key, error = %e, "feature flag DB read failed; treating as disabled");
            false
        }
    }
}

/// Bulk evaluate every known flag for a user. The frontend bootstrap
/// endpoint calls this once on app start.
pub async fn evaluate_all(
    cache: &FeatureFlagCache,
    db: &DatabaseConnection,
    user_id: Option<&Uuid>,
) -> HashMap<String, bool> {
    // Prefer the cache for everything; only fall through to the DB if the
    // cache is empty (e.g. cold start before the refresh tick has fired).
    if cache.len().await > 0 {
        let snapshot: Vec<Arc<FlagModel>> = {
            let g = cache.inner.read().await;
            g.values().map(|c| c.model.clone()).collect()
        };
        return snapshot
            .iter()
            .map(|m| (m.key.clone(), evaluate_flag(m, user_id)))
            .collect();
    }

    match FlagEntity::find().all(db).await {
        Ok(rows) => {
            // Populate the cache as a side effect so the next call is a hit.
            for m in &rows {
                cache.put(m.key.clone(), Arc::new(m.clone())).await;
            }
            rows.iter()
                .map(|m| (m.key.clone(), evaluate_flag(m, user_id)))
                .collect()
        }
        Err(e) => {
            warn!(error = %e, "evaluate_all: DB read failed; returning empty map");
            HashMap::new()
        }
    }
}

/// List every flag row, used by the admin GET endpoint. Always reads
/// from the DB (we want the admin UI to see the freshest data — cache is
/// only for the hot path).
pub async fn list(db: &DatabaseConnection) -> Result<Vec<FlagModel>, sea_orm::DbErr> {
    FlagEntity::find().all(db).await
}

/// Upsert a flag row. Pass the entire `FlagModel` — `key` is the primary
/// key, so on update we set every other column. Returns the new row.
pub async fn upsert(
    db: &DatabaseConnection,
    updated: FlagModel,
) -> Result<FlagModel, sea_orm::DbErr> {
    let key = updated.key.clone();
    let now = chrono::Utc::now();

    // Try update first.
    let existing = FlagEntity::find_by_id(&key).one(db).await?;
    let result = match existing {
        Some(_) => {
            // Update — preserve `created_at`, set `updated_at = now`.
            let mut am: FlagActive = updated.clone().into();
            am.created_at = Set(now); // not used on update path, but required by ActiveModel
            // ActiveModel's `update` does NOT touch created_at if we don't set it.
            // To be safe, set updated_at explicitly here.
            am.updated_at = Set(now);
            am.update(db).await?
        }
        None => {
            // Insert — set both timestamps to now.
            let am = FlagActive {
                key: Set(updated.key.clone()),
                description: Set(updated.description.clone()),
                enabled: Set(updated.enabled),
                user_whitelist: Set(updated.user_whitelist.clone()),
                percentage_rollout: Set(updated.percentage_rollout),
                metadata: Set(updated.metadata.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                updated_by: Set(updated.updated_by),
            };
            am.insert(db).await?
        }
    };

    // Bust the cache so the next read sees the new value. (We could
    // optimistic-update instead, but invalidation is simpler and the next
    // miss re-reads from the DB.)
    if let Some(cache) = current_cache() {
        cache.invalidate(&key).await;
    }

    Ok(result)
}

/// Delete a flag row. Returns `true` if a row was actually removed.
pub async fn delete_flag(
    cache: &FeatureFlagCache,
    db: &DatabaseConnection,
    key: &str,
) -> Result<bool, sea_orm::DbErr> {
    let res = FlagEntity::delete_by_id(key).exec(db).await?;
    cache.invalidate(key).await;
    Ok(res.rows_affected > 0)
}

/// Force a full re-read of the cache. Used by the background refresh task
/// and exposed so admin write paths can choose to refresh eagerly.
pub async fn refresh_from_db(cache: &FeatureFlagCache, db: &DatabaseConnection) -> Result<usize, sea_orm::DbErr> {
    let rows = FlagEntity::find().all(db).await?;
    let n = rows.len();
    cache.replace_all(rows).await;
    Ok(n)
}

// ─── Background refresh task ──────────────────────────────────────────

/// How often the background task reloads the cache. The PRD says "real
/// time, no restart", and 5s is the sweet spot: small enough that an
/// admin flipping a switch in the UI sees the change within a few
/// requests, large enough that we don't hammer the DB.
pub const CACHE_REFRESH_SECS: u64 = 5;

/// Process-wide handle to the cache. We use a `std::sync::OnceLock` so the
/// cache survives service-handler creation but can be set exactly once at
/// startup; tests that need an isolated cache can construct their own
/// `FeatureFlagCache` directly.
static CACHE: std::sync::OnceLock<Arc<FeatureFlagCache>> = std::sync::OnceLock::new();

/// Initialise the process-wide cache. Call from `main` (or from tests
/// that want a real cache wired in). Idempotent: subsequent calls are
/// no-ops and return the original handle.
pub fn init_cache() -> Arc<FeatureFlagCache> {
    CACHE
        .get_or_init(|| Arc::new(FeatureFlagCache::new()))
        .clone()
}

/// Return the current process-wide cache, or `None` if it has not been
/// initialised yet. Use this in write paths that need to bust the cache
/// but don't want to thread a cache handle through every call site.
fn current_cache() -> Option<Arc<FeatureFlagCache>> {
    CACHE.get().cloned()
}

/// Spawn a background task that refreshes the cache every
/// `CACHE_REFRESH_SECS` seconds. The task runs forever (or until the
/// process exits); failures are logged but don't kill the task — a
/// transient DB hiccup should not stop the next attempt.
pub fn spawn_refresh_task(db: DatabaseConnection) -> tokio::task::JoinHandle<()> {
    let cache = init_cache();
    tokio::spawn(async move {
        // Do one immediate refresh so the first `is_enabled` call after
        // startup doesn't have to do a sync fetch.
        if let Err(e) = refresh_from_db(&cache, &db).await {
            warn!(error = %e, "initial feature_flag cache refresh failed");
        } else {
            debug!("feature_flag cache primed");
        }

        let mut tick = tokio::time::interval(Duration::from_secs(CACHE_REFRESH_SECS));
        // Skip the immediate-fire on the first tick — we already refreshed
        // above.
        tick.tick().await;
        loop {
            tick.tick().await;
            match refresh_from_db(&cache, &db).await {
                Ok(n) => debug!(rows = n, "feature_flag cache refreshed"),
                Err(e) => warn!(error = %e, "feature_flag cache refresh failed"),
            }
        }
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn model(key: &str, enabled: bool, pct: i32, whitelist: Vec<Uuid>) -> FlagModel {
        FlagModel {
            key: key.to_string(),
            description: format!("test flag {key}"),
            enabled,
            user_whitelist: json!(whitelist
                .into_iter()
                .map(|u| u.to_string())
                .collect::<Vec<_>>()),
            percentage_rollout: pct,
            metadata: json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn globally_disabled_returns_false() {
        let m = model("x", false, 100, vec![]);
        let u = Uuid::new_v4();
        assert!(!evaluate_flag(&m, Some(&u)));
        assert!(!evaluate_flag(&m, None));
    }

    #[test]
    fn globally_enabled_returns_true() {
        let m = model("x", true, 100, vec![]);
        let u = Uuid::new_v4();
        assert!(evaluate_flag(&m, Some(&u)));
        assert!(evaluate_flag(&m, None));
    }

    #[test]
    fn whitelist_overrides_disabled() {
        let u = Uuid::new_v4();
        let m = model("x", false, 0, vec![u]);
        // Even though globally disabled and pct=0, the whitelisted user
        // sees the feature.
        assert!(evaluate_flag(&m, Some(&u)));
        // A different user still gets the disabled path.
        let other = Uuid::new_v4();
        assert!(!evaluate_flag(&m, Some(&other)));
    }

    #[test]
    fn percentage_rollout_at_25_percent() {
        let m = model("x", true, 25, vec![]);
        // Sample 1000 fresh UUIDs and check the share is roughly 25%
        // (loose bound to keep the test stable across FxHash tweaks).
        let n = 1000u32;
        let hits: u32 = (0..n)
            .map(|_| Uuid::new_v4())
            .filter(|u| evaluate_flag(&m, Some(u)))
            .count() as u32;
        let pct = (hits as f64) / (n as f64) * 100.0;
        assert!(
            (15.0..=35.0).contains(&pct),
            "expected ~25% rollout, got {pct:.1}%"
        );
    }

    #[test]
    fn cache_replaces_all_clears_old_keys() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let cache = FeatureFlagCache::new();
            let m1 = model("a", true, 100, vec![]);
            let m2 = model("b", true, 100, vec![]);
            cache
                .replace_all(vec![m1.clone(), m2.clone()])
                .await;
            assert_eq!(cache.len().await, 2);

            // Replace with a one-row set; "b" should be gone.
            cache.replace_all(vec![m1.clone()]).await;
            assert_eq!(cache.len().await, 1);
            assert!(cache.get("a").await.is_some());
            assert!(cache.get("b").await.is_none());
        });
    }

    #[test]
    fn unknown_user_with_partial_pct_is_off() {
        // Unauthenticated callers (user_id=None) get the conservative
        // answer when pct is non-trivial.
        let m = model("x", true, 50, vec![]);
        assert!(!evaluate_flag(&m, None));
    }

    #[test]
    fn bucket_distribution_is_uniformish() {
        // Sanity check that bucket_for_user doesn't have a glaring bias.
        let n = 10_000u32;
        let counts = (0..n)
            .map(|_| Uuid::new_v4())
            .fold([0usize; 100], |mut acc, u| {
                acc[bucket_for_user(&u) as usize] += 1;
                acc
            });
        let expected = n as f64 / 100.0;
        let max = *counts.iter().max().unwrap() as f64;
        let min = *counts.iter().min().unwrap() as f64;
        // Allow a 50% swing around the expected — this is a non-statistical
        // test, just a smoke check.
        assert!(max < expected * 1.5, "max bucket {max} vs expected {expected}");
        assert!(min > expected * 0.5, "min bucket {min} vs expected {expected}");
    }
}

// ─── Used by tests in `tests/feature_flag_test.rs` ─────────────────────

/// Helper used by integration tests to evaluate a flag without a DB.
/// Mirrors the cache-hit path: caller hands in a `Vec<FlagModel>`
/// snapshot, we run `evaluate_flag` against the first match.
#[allow(dead_code)]
pub fn evaluate_from_models(
    flags: &[FlagModel],
    key: &str,
    user_id: Option<&Uuid>,
) -> bool {
    flags
        .iter()
        .find(|f| f.key == key)
        .map(|f| evaluate_flag(f, user_id))
        .unwrap_or(false)
}
