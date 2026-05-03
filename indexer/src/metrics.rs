//! Prometheus metrics (optional `GET /metrics` on the **dedicated** `METRICS_BIND` listener when non-empty).
//! Production deploys forbid `0.0.0.0` / `::` on that listener; see [`Config`](crate::config::Config) and `docs/operator-secrets.md`.

use std::sync::OnceLock;

use prometheus::{register_int_counter, IntCounter};

fn blocks_processed() -> &'static IntCounter {
    static C: OnceLock<IntCounter> = OnceLock::new();
    C.get_or_init(|| {
        register_int_counter!(
            "indexer_blocks_processed_total",
            "Number of chain blocks processed by the indexer"
        )
        .expect("register indexer_blocks_processed_total")
    })
}

fn block_process_errors() -> &'static IntCounter {
    static C: OnceLock<IntCounter> = OnceLock::new();
    C.get_or_init(|| {
        register_int_counter!(
            "indexer_block_process_errors_total",
            "Errors while processing a block (parser or DB update failures)"
        )
        .expect("register indexer_block_process_errors_total")
    })
}

fn block_time_fallbacks() -> &'static IntCounter {
    static C: OnceLock<IntCounter> = OnceLock::new();
    C.get_or_init(|| {
        register_int_counter!(
            "indexer_block_time_fallbacks_total",
            "Block timestamps missing or invalid; UTC now used (candle skew risk)"
        )
        .expect("register indexer_block_time_fallbacks_total")
    })
}

pub fn inc_blocks_processed() {
    blocks_processed().inc();
}

pub fn inc_block_process_errors() {
    block_process_errors().inc();
}

pub fn inc_block_time_fallbacks() {
    block_time_fallbacks().inc();
}

pub fn gather_text() -> Result<Vec<u8>, prometheus::Error> {
    // Ensure counters are registered before gather (OnceLock registers on first get_or_init).
    let _ = blocks_processed();
    let _ = block_process_errors();
    let _ = block_time_fallbacks();

    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buf = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_contains_indexer_counters() {
        inc_blocks_processed();
        let text = String::from_utf8(gather_text().expect("gather")).expect("utf8");
        assert!(text.contains("indexer_blocks_processed_total"));
    }
}
