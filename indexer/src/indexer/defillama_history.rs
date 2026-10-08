//! Explicit bounded repair of old Llama materializations (#1354).
//!
//! Replays the existing ledger, never reprices events or resamples historical auxiliary asset snapshots.
//! Missing historical marks/supply remain unavailable; upstream charts are separate.
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;

use super::defillama::ADAPTER_START_UTC_DAY;

pub const MAX_HISTORY_DAYS: i64 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl HistoryRange {
    pub fn parse(start: &str, end: &str, today: NaiveDate) -> anyhow::Result<Self> {
        let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("start must be a UTC date (YYYY-MM-DD)"))?;
        let end = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("end must be a UTC date (YYYY-MM-DD)"))?;
        let first = chrono::DateTime::from_timestamp(ADAPTER_START_UTC_DAY, 0)
            .expect("adapter start is valid")
            .date_naive();
        anyhow::ensure!(
            start >= first
                && end >= start
                && end < today
                && (end - start).num_days() < MAX_HISTORY_DAYS,
            "require 1-400 completed UTC days at/after 2026-08-17"
        );
        Ok(Self { start, end })
    }

    pub fn days(self) -> i64 {
        (self.end - self.start).num_days() + 1
    }
}

/// Every day commits atomically. Retry the same range after interruption; UPSERT
/// rebuilds all source rows, including newly idle days, without touching stamps.
pub async fn repair_history(pool: &PgPool, range: HistoryRange) -> Result<i64, sqlx::Error> {
    for offset in 0..range.days() {
        let day = range.start + Duration::days(offset);
        let start = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).expect("valid UTC day"));
        crate::db::queries::defillama::repair_defillama_day(pool, start).await?;
        tracing::info!(utc_day = %day, "DeFiLlama history materialized from existing ledger");
    }
    Ok(range.days())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_completed_history_only() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let range = HistoryRange::parse("2026-08-17", "2026-09-24", today).unwrap();
        assert_eq!(range.days(), 39);
        for (start, end) in [
            ("2026-08-16", "2026-09-24"),
            ("2026-08-17", "2026-10-08"),
            ("2026-09-24", "2026-08-17"),
            ("2026-08-17; DROP TABLE", "2026-09-24"),
            ("2026-02-30", "2026-09-24"),
        ] {
            assert!(HistoryRange::parse(start, end, today).is_err());
        }
        let later = NaiveDate::from_ymd_opt(2028, 1, 1).unwrap();
        assert!(HistoryRange::parse("2026-08-17", "2027-09-20", later).is_ok());
        assert!(HistoryRange::parse("2026-08-17", "2027-09-21", later).is_err());
    }
}
