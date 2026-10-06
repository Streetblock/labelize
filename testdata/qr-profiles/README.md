# Q01 profile matrices

Input: `^XA^CI27^FO24,24^BQN,2,4^FDQM,B002012345678901234567890^FS^XZ`.
Rows contain only symbol modules (`1` dark), without quiet zones or scaling.

- `q01-labelary.txt`: independent Labelary API response captured in the
  2026-10-06 comparison study. Numeric, V1, EC-Q, mask 0. Request:
  `https://api.labelary.com/v1/printers/12dpmm/labels/2x1/0/` (2026-10-06 13:52:49 UTC).
- `q01-explicit-byte.txt`: recorded PR52 encoder output from that study. Byte,
  V2, EC-Q, mask 2; the combined PR52/PR53 study output has the same matrix.
  The user visually confirmed this matrix against the native Zebra comparison
  print. It is a regression baseline for that observation, **not** an
  independently decoded full printer scan or an ISO reference implementation.

`tests/unit_compatibility_profiles.rs` compares both complete matrices, then
uses an independent decoder to verify their mode, EC level and equal payload.
See [profile evidence limits](../../docs/COMPATIBILITY_PROFILES.md).
