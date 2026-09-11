# Historical Data Matrix ECC 000-140

Development checkpoint: the encoder now covers all five historical ECC levels.
The intended upstream submission is one combined PR
after the remaining field handling has been implemented and
reviewed. This document describes the current checkpoint, not the final PR scope.

This change adds a Legacy encoder path. Omitted/empty ZPL `^BX` quality
and explicit `0`/`000` produce ECC 000. Explicit `200` and EPL retain ECC 200.
ECC 050, 080, 100 and 140 use their respective convolutional encoders;
they never silently produce an ECC 200 symbol.

## Why support a removed variant?

ISO/IEC 16022:2024 removed the historical ECC 000-140 family. That does not
remove existing printer commands. Zebra still documents these qualities and
default quality 0 in its BX programming reference; the contributor also
observed Legacy symbols printing on a contemporary Zebra 421-class printer.
This is compatibility work for existing ZPL, not a recommendation to select
Legacy for a new application. It does not assert support on every current
Zebra model or firmware.

- [ISO/IEC 16022:2024](https://www.iso.org/standard/80926.html)
- [2024 foreword, published standard preview](https://gso-sims-preview-doc-aws.s3-eu-west-1.amazonaws.com/iso-iec-16022-2024-en.html)
- [Zebra BX reference](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html)

## Norm-first implementation and provenance

The implementation was developed directly from the historical standard:
ISO/IEC 16022:2000, the FCD draft for the following edition, and the GOST R
ISO/IEC 16022-2008 adoption. Relevant material: Annex H (placement), Annex I
(character formats), Annex J (CRC), Annex K (convolution), Annex L (randomization) and Annex Q
(example intermediate values). The 2024 edition is not the specification
for these removed Legacy variants.

The Rust encoder and placement function are ported from the author's own
QR Atelier JavaScript implementation:

- Repository: https://github.com/Streetblock/QR-Atelier
- Commit: `432aa76de2116adc055062f91ef622f4dc9568cb`
- Files: `libs/DMlegacy.js`, `libs/DMlegacyPlacementGenerator.js`
- Copyright 2026 David Block; MIT option of MIT OR Apache-2.0.
- License: [QR-Atelier-MIT.txt](../licenses/QR-Atelier-MIT.txt).

Rust-specific changes use byte slices, a bounded lazy cache and Labelize's
BitMatrix rather than JavaScript strings and arrays. The source commit already
contains the norm-confirmed random byte BC at offset 211; there is no separate
Rust correction relative to that source. The placement formula uses filtered
bit reversal, inverse permutation, cyclic row shifts and corner swaps. It was
empirically reconstructed and checked exhaustively for the 21 supported sizes,
not supplied as a normative algorithm or proven beyond that finite domain.

## Evidence and its limits

There is no independent overall verification of this encoder. Tests and source
review must not be described as third-party certification or validation by an
independently developed Legacy library. Comparisons with our JavaScript encoder
and our own Decoder can share the same interpretation mistakes.

The evidence is recorded separately:

- `testdata/legacy/placement-grids.txt`: all 21 historical placement grids,
  18,389 cells, retained as fixed test-only reference data. They were extracted
  from the FCD PDF with two methods; those methods share the same source. Visual
  checks of other editions supplement that work, not an independent encoder.
- Fixed base-format examples and the CRC 7559 for format 3 / AB12-X check
  individual norm-derived stages, not the whole ECC 000 output independently.
- `ecc000-printer-matrices.txt`: three recorded module matrices (200, 300 and
  400 digits; complete sides 31, 35 and 41). The 200-digit case was obtained
  from the user's Zebra photo; the older 300/400 records do not independently
  identify their encoder. These fixtures establish exact equality for those
  observations only. No new printer run was performed for this Rust port.
- `ecc000-js-port-vectors.txt`: twelve fixed outputs from the same author's
  JavaScript implementation, spanning six formats and automatic/49-module
  sizes. These are port-regression fixtures, explicitly not independent evidence.
- `ecc050-annex-q-matrix.txt`: the published 13x13 ECC 050 / format 3 / AB12-X
  example. Its 96 protected bits are also tested separately. This is a fixed
  example from Annex Q, visually checked in FCD and GOST; two editions of the
  same example do not constitute two independently developed implementations.
- `convolution-js-stages.txt`: 200 same-source JS vectors, input lengths 0..49
  for each convolutional quality, covering partial groups and zero flushing.
- `convolution-js-matrices.txt`: 48 same-source JS matrices, all four qualities
  and six formats, each automatically sized and forced to 49x49. Format 6
  includes 00/80/FF/96/01 bytes. These detect port regressions, not independent
  correctness. No new printer observations are claimed for these four levels.

The convolution matrix fixture headers are `quality|format|side|payload_hex`,
followed by binary module rows; blank lines separate cases. Stage fixtures are
`quality|input_bits|protected_bits`. They were generated once from the pinned
QR-Atelier commit above, not regenerated by Rust tests. For reproducibility,
stage input bit `i` is 1 when `(i*i + 3*i + 7) % 11 < 5`, otherwise 0; input
lengths run from 0 through 49. The JS exports are `encodeLegacyEcc050/080/100/140`
and `generateLegacyDataMatrix`. The latter's six format inputs are recorded as
hex in each matrix header; each is generated with automatic size and size 49.

Source PDF SHA-256 values (documents are not distributed in the repository):

- FCD: `c618f525c53d884614237349e0e83a85dd4378bca6ad64982d484bec6242fcaa`
- GOST: `ce038dcde4cd82c8626b9782abb2c379868f7d9bd709dd836cd69c5459b9888d`
- ISO 2000: `3726076a792673241f9dfef2253e6e1dc815573e3fd564cd176638001ba2d317`

## Scope and deliberate errors

The raw `barcodes::datamatrix_legacy::encode` API retains ECC 000;
`encode_with_ecc` additionally takes quality 0, 50, 80, 100 or 140. Both accept
literal bytes, explicit format 1..6 and an optional complete odd symbol size.
They implement the CRC, 9-bit length field, six encodations, ECC header,
convolution where applicable, zero fill, randomization,
placement and finder border. It rejects empty input, lengths above 511 and
content that does not fit; it never truncates or switches ECC. The 511 bound is
the implemented historical record-length limit, not a claim that every printer
accepts all payloads up to that length. Zebra's documented 596-character field
limit does not override the smaller record/symbol limits implemented here.

ZPL currently supports ASCII field data (including ASCII control bytes from
existing FH processing). Non-ASCII ZPL text and Legacy field escape sequences
(backslash-ampersand, doubled backslash, double pipe) return explicit errors;
those field/byte-preservation rules remain follow-up work. They are available
as literal byte data through the raw API, without ZPL interpretation. The
ECC 200 escape-selector parameter has no effect on Legacy field contents.

Dimensions follow Zebra's documented Legacy rule: square symbols, the larger
requested rows/columns, values above 49 treated as automatic, invalid small or
even dimensions rejected. An explicit rectangular aspect request is rejected.
Existing rendering rotation, field positioning and module scaling are reused.
CV diagnostic printing remains outside this checkpoint.

Convolution protects the format/CRC/length/data record, excluding the ECC
header. The final input group is zero-padded before the state is flushed.
Only then are the header and remaining zero fill added and the complete data
area randomized. Each encode call starts with an all-zero state.

| ECC | Input bits/cycle | Output bits/cycle | Flush cycles | Minimum complete side | Maximum format-6 bytes at 49x49 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 000 | 1 | 1 | 0 | 9 | 271 |
| 050 | 3 | 4 | 3 | 11 | 200 |
| 080 | 2 | 3 | 11 | 13 | 176 |
| 100 | 1 | 2 | 15 | 13 | 131 |
| 140 | 1 | 4 | 13 | 17 | 63 |

Capacity tests assert both the last fitting byte count and rejection of the
next byte; these are encoder capacities, not general printer field limits.
ECC 100 retains the diagram correction already present in the attributed
source: no delay-12 tap. A fixed impulse test exercises all fifteen delays.

The frozen reference grids are compiled only into tests; production generates
and caches each requested grid once. No JavaScript runtime or encoder dependency
is introduced.
