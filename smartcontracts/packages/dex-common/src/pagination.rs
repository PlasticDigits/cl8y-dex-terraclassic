pub const DEFAULT_LIMIT: u32 = 30;
pub const MAX_LIMIT: u32 = 100;

pub fn calc_limit(limit: Option<u32>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}
