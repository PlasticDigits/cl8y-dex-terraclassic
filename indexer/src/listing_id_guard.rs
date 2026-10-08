//! Opaque listing ids are never contract addresses (#1437).
//!
//! Keep this small guard outside listing adapters so discovery and ordinary pair
//! lookups reject ids before database work or LCD requests. Case-insensitive
//! matching also rejects attacker-controlled uppercase variants.
pub fn is_listing_only_id(address: &str) -> bool {
    address
        .get(.."cl8y-virtual:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("cl8y-virtual:"))
}

#[cfg(test)]
mod tests {
    use super::is_listing_only_id;

    #[test]
    fn rejects_reserved_namespace_including_injection_variants() {
        for id in [
            "cl8y-virtual:clunc-uluna",
            "cl8y-virtual:cl8y-virtual:clunc-uluna",
            "CL8Y-VIRTUAL:clunc-uluna",
            "cl8y-virtual:/terra1contract",
            "cl8y-virtual:%2Fterra1contract",
        ] {
            assert!(is_listing_only_id(id));
        }
        assert!(!is_listing_only_id("terra1contract"));
        assert!(!is_listing_only_id("cl8y"));
        assert!(!is_listing_only_id("éééééééé"));
    }
}
