-- Speed NOT EXISTS (placements vs cancellations) on GET .../limit-placements (GitLab #135).
CREATE INDEX IF NOT EXISTS idx_lo_cancellations_pair_order ON limit_order_cancellations (pair_id, order_id);
