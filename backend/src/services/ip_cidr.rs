//! services/ip_cidr.rs — IPv4/IPv6 parsing + CIDR matching utilities
//!
//! Used by P3-A (admin IP whitelist) to:
//!   1. Parse a user-supplied string into an `IpNetwork` (validate input)
//!   2. Decide whether a peer IP falls inside any of a user's allow-listed
//!      CIDR ranges.
//!
//! Design:
//!   - We accept "single host" shorthand: "192.168.1.1" (no `/`) is normalised
//!     to "192.168.1.1/32" for v4 or "::1/128" for v6 — same semantics, no
//!     surprises in the DB.
//!   - All errors are `AppError::Validation` so the HTTP layer maps them to
//!     400 without further work.
//!   - The matcher is allocation-free: it iterates CIDR strings, parses
//!     each lazily, and short-circuits on the first hit. This keeps the
//!     admin-route hot path cheap even for admins with a dozen rules.

use ipnetwork::IpNetwork;
use std::net::IpAddr;
use std::str::FromStr;

use crate::utils::error::AppError;

/// Parse a user-supplied IP/CIDR string into a normalised `IpNetwork`.
///
/// Accepts:
///   - "192.168.1.0/24"  (CIDR, v4)
///   - "2001:db8::/32"   (CIDR, v6)
///   - "10.0.0.5"        (single v4 host — normalised to /32)
///   - "2001:db8::1"     (single v6 host — normalised to /128)
///
/// Returns `Validation` on:
///   - empty string
///   - unparsable input
///   - addresses with leading/trailing whitespace
pub fn parse_cidr(input: &str) -> Result<IpNetwork, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("ip_cidr cannot be empty".into()));
    }
    // First try strict CIDR (with `/`). If that fails, try as a bare host and
    // normalise — the user shouldn't have to know the /32 vs /128 difference.
    if let Ok(net) = IpNetwork::from_str(trimmed) {
        return Ok(net);
    }
    if let Ok(addr) = IpAddr::from_str(trimmed) {
        return Ok(IpNetwork::from(addr));
    }
    Err(AppError::Validation(format!(
        "invalid ip_cidr '{}': expected IPv4/IPv6 address or CIDR (e.g. 192.168.1.0/24)",
        input
    )))
}

/// Render an `IpNetwork` back to its canonical string form. This is what we
/// store in the DB so a single host comes back as "x.x.x.x/32" (or v6
/// equivalent) — keeps equality checks deterministic.
pub fn canonicalise(net: IpNetwork) -> String {
    net.to_string()
}

/// Does `ip` fall inside any of the CIDR strings?
///
/// The list of CIDRs is iterated, parsing lazily. On the first hit the
/// function returns `true`; if all fail to parse, the row is skipped (so a
/// malformed entry in the DB never wedges a request — the middleware
/// surfaces a 500 in that case via `match_cidrs_strict`).
pub fn ip_matches_any(ip: IpAddr, cidrs: &[String]) -> bool {
    cidrs.iter().any(|c| match_cidr(ip, c))
}

pub fn match_cidr(ip: IpAddr, cidr: &str) -> bool {
    IpNetwork::from_str(cidr)
        .map(|net| net.contains(ip))
        .unwrap_or(false)
}

/// Strict variant: returns `Err` on malformed CIDR rows (call this from the
/// middleware so DB corruption is loud, not silent).
pub fn match_cidrs_strict(ip: IpAddr, cidrs: &[String]) -> Result<bool, AppError> {
    for c in cidrs {
        let net = IpNetwork::from_str(c.trim()).map_err(|e| {
            AppError::Internal(format!("malformed ip_cidr row in DB: '{}' ({})", c, e))
        })?;
        if net.contains(ip) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn parse_cidr_accepts_v4_cidr() {
        let n = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(n.to_string(), "192.168.1.0/24");
    }

    #[test]
    fn parse_cidr_normalises_single_host() {
        let n = parse_cidr("10.0.0.5").unwrap();
        assert_eq!(n.to_string(), "10.0.0.5/32");
    }

    #[test]
    fn parse_cidr_rejects_empty() {
        assert!(parse_cidr("").is_err());
        assert!(parse_cidr("   ").is_err());
    }

    #[test]
    fn parse_cidr_rejects_garbage() {
        assert!(parse_cidr("not-an-ip").is_err());
    }

    #[test]
    fn match_cidr_v4_inside_range() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42));
        assert!(match_cidr(ip, "192.168.1.0/24"));
        assert!(!match_cidr(ip, "10.0.0.0/8"));
    }

    #[test]
    fn match_cidr_v4_single_host() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        assert!(match_cidr(ip, "10.0.0.5/32"));
        assert!(!match_cidr(ip, "10.0.0.6/32"));
    }

    #[test]
    fn ip_matches_any_short_circuits() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let cidrs = vec!["192.168.0.0/16".into(), "10.0.0.0/8".into()];
        assert!(ip_matches_any(ip, &cidrs));
    }

    #[test]
    fn ip_matches_any_no_match() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let cidrs = vec!["192.168.0.0/16".into(), "10.0.0.0/8".into()];
        assert!(!ip_matches_any(ip, &cidrs));
    }

    #[test]
    fn match_cidrs_strict_propagates_parse_error() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let cidrs = vec!["not-a-cidr".into()];
        assert!(match_cidrs_strict(ip, &cidrs).is_err());
    }

    #[test]
    fn parse_cidr_v6_works() {
        let n = parse_cidr("2001:db8::1").unwrap();
        assert_eq!(n.to_string(), "2001:db8::1/128");
        let ip = IpAddr::from_str("2001:db8::42").unwrap();
        assert!(match_cidr(ip, "2001:db8::/32"));
        assert!(!match_cidr(ip, "2001:db9::/32"));
    }
}
