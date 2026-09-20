//! Select + parse baked commit env for generic `GET /health` (`git_sha`).
//!
//! Invariants **H1276-2**: helpers take `&str` / `Option<&str>` (no process env).
//! Do not implement “parse fail → try the other env.”

const SECRET_PREFIXES: &[&str] = &[
    "Bearer ", "bearer ", "sk-", "ghp_", "gho_", "ghu_", "ghs_", "glpat-", "N|",
];

fn trim_ascii(raw: &str) -> &str {
    raw.trim_matches(|c: char| c.is_ascii_whitespace())
}

/// Use `GIT_SHA` only when present and non-empty after ASCII trim; else `SOURCE_COMMIT`.
pub(crate) fn select_commit_env<'a>(
    git_sha: Option<&'a str>,
    source_commit: Option<&'a str>,
) -> Option<&'a str> {
    match git_sha {
        Some(value) if !trim_ascii(value).is_empty() => Some(value),
        _ => source_commit,
    }
}

/// Parse the **selected** candidate. Omit rather than echo `HEAD` / secrets / non-hex.
pub(crate) fn parse_git_sha(raw: &str) -> Option<String> {
    let trimmed = trim_ascii(raw);
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().any(|c| c.is_ascii_whitespace()) {
        return None;
    }
    if trimmed.contains('=') {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("HEAD") {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("main")
        || trimmed.eq_ignore_ascii_case("refs/heads/main")
        || (trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("refs/"))
    {
        return None;
    }
    if SECRET_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let n = lower.len();
    if !(7..=40).contains(&n) {
        return None;
    }
    if !lower
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    Some(lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";
    const HEX7: &str = "abcdef0";

    #[test]
    fn select_empty_git_sha_falls_through() {
        assert_eq!(select_commit_env(Some(""), Some(HEX40)), Some(HEX40));
        assert_eq!(select_commit_env(Some("   "), Some(HEX40)), Some(HEX40));
        assert_eq!(select_commit_env(None, Some(HEX40)), Some(HEX40));
        assert_eq!(select_commit_env(None, None), None);
        assert_eq!(select_commit_env(Some(""), None), None);
    }

    #[test]
    fn select_non_empty_rejected_does_not_fall_through() {
        assert_eq!(select_commit_env(Some("HEAD"), Some(HEX40)), Some("HEAD"));
        assert_eq!(select_commit_env(Some("main"), Some(HEX40)), Some("main"));
        assert_eq!(
            select_commit_env(Some("refs/heads/main"), Some(HEX40)),
            Some("refs/heads/main")
        );
    }

    #[test]
    fn parse_valid_hex_and_uppercase_normalize() {
        assert_eq!(parse_git_sha(HEX40).as_deref(), Some(HEX40));
        assert_eq!(parse_git_sha(HEX7).as_deref(), Some(HEX7));
        assert_eq!(
            parse_git_sha("ABCDEF0123456789ABCDEF0123456789ABCDEF01").as_deref(),
            Some("abcdef0123456789abcdef0123456789abcdef01")
        );
    }

    #[test]
    fn parse_trailing_newline_on_40_char_hex_is_present() {
        let with_nl = format!("{HEX40}\n");
        assert_eq!(parse_git_sha(&with_nl).as_deref(), Some(HEX40));
    }

    #[test]
    fn parse_head_main_refs_omitted() {
        assert!(parse_git_sha("HEAD").is_none());
        assert!(parse_git_sha("head").is_none());
        assert!(parse_git_sha("main").is_none());
        assert!(parse_git_sha("MAIN").is_none());
        assert!(parse_git_sha("refs/heads/main").is_none());
        assert!(parse_git_sha("REFS/HEADS/MAIN").is_none());
        assert!(parse_git_sha("refs/heads/issue/1276").is_none());
    }

    #[test]
    fn parse_length_bounds() {
        assert!(parse_git_sha("abcdef").is_none()); // 6
        assert!(parse_git_sha("abcdef0").is_some()); // 7
        let len41 = format!("{HEX40}a");
        assert_eq!(len41.len(), 41);
        assert!(parse_git_sha(&len41).is_none());
    }

    #[test]
    fn parse_empty_whitespace_internal_equals() {
        assert!(parse_git_sha("").is_none());
        assert!(parse_git_sha("   ").is_none());
        assert!(parse_git_sha("abc def0").is_none());
        assert!(parse_git_sha("GIT_SHA=abcdef0").is_none());
        assert!(parse_git_sha("abcdef0=foo").is_none());
    }

    #[test]
    fn parse_secret_prefixes_omitted() {
        assert!(parse_git_sha("Bearer abcdef0").is_none());
        assert!(parse_git_sha("bearer abcdef0").is_none());
        assert!(parse_git_sha("sk-abcdef0123456789").is_none());
        assert!(parse_git_sha("ghp_abcdef0123456789").is_none());
        assert!(parse_git_sha("gho_abcdef0123456789").is_none());
        assert!(parse_git_sha("ghu_abcdef0123456789").is_none());
        assert!(parse_git_sha("ghs_abcdef0123456789").is_none());
        assert!(parse_git_sha("glpat-abcdef01234567").is_none());
        assert!(parse_git_sha("N|abcdef0123456789abcdef").is_none());
    }

    #[test]
    fn parse_non_hex_omitted() {
        assert!(parse_git_sha("zzzzzzz").is_none());
        assert!(parse_git_sha("abcdefg").is_none());
    }
}
