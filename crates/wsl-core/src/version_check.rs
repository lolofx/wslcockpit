/// Découpe `v1.2.3` ou `1.2.3-beta` en (1, 2, 3). Les parties absentes valent 0.
pub fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let core = s.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().map_or(Some(0), |v| v.parse().ok())?;
    let patch = it.next().map_or(Some(0), |v| v.parse().ok())?;
    Some((major, minor, patch))
}

/// Vrai si `latest` est strictement plus récente que `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("0.2"), Some((0, 2, 0)));
        assert_eq!(parse_semver("1.0.0-beta.1"), Some((1, 0, 0)));
        assert_eq!(parse_semver("n'importe quoi"), None);
    }

    #[test]
    fn comparaison() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("", "0.1.0"));
    }
}
