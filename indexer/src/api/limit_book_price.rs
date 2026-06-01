//! Positive decimal string compare for limit-book ordering (matches dApp `comparePositiveDecimalStrings`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalCmp {
    Lt,
    Eq,
    Gt,
}

#[derive(Debug, Clone)]
struct UnsignedParts {
    int: String,
    frac: String,
}

fn split_positive_decimal(s: &str) -> Option<UnsignedParts> {
    let t = s.trim();
    if t.is_empty() || t.starts_with('-') {
        return None;
    }
    if !t.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let parts: Vec<&str> = t.split('.').collect();
    if parts.len() > 2 {
        return None;
    }
    let int_raw = parts.first().copied().unwrap_or("");
    let frac_raw = parts.get(1).copied().unwrap_or("");
    if int_raw.is_empty() && frac_raw.is_empty() {
        return None;
    }
    if !int_raw.is_empty() && !int_raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac_raw.is_empty() && !frac_raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let int = if int_raw.is_empty() {
        "0".to_string()
    } else {
        let stripped = int_raw.trim_start_matches('0');
        if stripped.is_empty() {
            "0".to_string()
        } else {
            stripped.to_string()
        }
    };
    Some(UnsignedParts {
        int,
        frac: frac_raw.to_string(),
    })
}

fn cmp_unsigned_parts(a: &UnsignedParts, b: &UnsignedParts) -> DecimalCmp {
    if a.int.len() != b.int.len() {
        return if a.int.len() < b.int.len() {
            DecimalCmp::Lt
        } else {
            DecimalCmp::Gt
        };
    }
    if a.int != b.int {
        return if a.int < b.int {
            DecimalCmp::Lt
        } else {
            DecimalCmp::Gt
        };
    }
    let max_f = a.frac.len().max(b.frac.len());
    let fa: String = format!("{:0<width$}", a.frac, width = max_f);
    let fb: String = format!("{:0<width$}", b.frac, width = max_f);
    if fa == fb {
        DecimalCmp::Eq
    } else if fa < fb {
        DecimalCmp::Lt
    } else {
        DecimalCmp::Gt
    }
}

/// Compare two positive decimal strings (no scientific notation). Returns `None` if invalid.
pub fn compare_positive_decimal_strings(a: &str, b: &str) -> Option<DecimalCmp> {
    let pa = split_positive_decimal(a)?;
    let pb = split_positive_decimal(b)?;
    Some(cmp_unsigned_parts(&pa, &pb))
}

/// Validate a limit-book price query value (positive decimal).
pub fn validate_limit_book_price(s: &str) -> Result<String, String> {
    let t = s.trim();
    if compare_positive_decimal_strings(t, "0") != Some(DecimalCmp::Gt) {
        return Err("price must be a positive decimal".to_string());
    }
    Ok(t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_like_frontend() {
        assert_eq!(
            compare_positive_decimal_strings("1.5", "1.40"),
            Some(DecimalCmp::Gt)
        );
        assert_eq!(
            compare_positive_decimal_strings("1.0", "1.0"),
            Some(DecimalCmp::Eq)
        );
        assert_eq!(
            compare_positive_decimal_strings("0.9", "1.0"),
            Some(DecimalCmp::Lt)
        );
        assert!(compare_positive_decimal_strings("-1", "1").is_none());
        assert!(compare_positive_decimal_strings("1e2", "1").is_none());
    }
}
