# 8654 — known-bad ALPHA taxed control

Columbus-5 `cw20-taxed` with `tax_map`. **Expected FAIL** on 1:1 and **P2**. If this ID (or the in-process FoT mutant) goes green, the harness is wrong (**C4**).

Do **not** whitelist. In-process oracle: `cw20_codeid_harness::mutant_a1_fot_breaks_one_to_one` and `layer_b_b1_fot_desyncs_reserves`.
