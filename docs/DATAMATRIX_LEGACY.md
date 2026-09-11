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
The ZPL field preprocessing is new Rust-side integration based on the Zebra
references below; it is not imported Legacy-escape functionality from QR Atelier
or the Toolkit, where those escape rules were still recorded as open work.

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
placement and finder border. They reject empty input, lengths above 511 and
content that does not fit; they never truncate or switch ECC. The 511 bound is
an implementation limitation pending clarification, not a claimed normative or
printer capacity. See the explicit long-record discrepancy below.

ZPL preserves a separate byte representation through command tokenization,
FH decoding, field resolution and stored-format recalls. Legacy rendering uses
these bytes, not the Unicode display string. This preserves invalid UTF-8 and
distinguishes, for example, FH E4 from FH C3 A4 even when the text decoder
displays both as the same character. Literal transport CR/LF/TAB are ignored
as before; FH can insert these bytes after tokenization. There is no additional
character-set transcoding of the transmitted Legacy bytes. Formats 1..5 still
reject bytes outside their respective repertoires.

Legacy fields pass their preserved bytes directly to the encoder after FH.
Backslash-ampersand, doubled backslashes and double pipes stay literal; only
explicit FH 0D/0A inserts CR/LF. Parameter g remains irrelevant for Legacy;
ECC200 has its own independent escape processing.

This follows the ZD421 (V93.21.17Z) L01-L04 printer observations: CI13 and CI27
produce identical matrices for all six ECC000/F6 probe fields. See
`examples/legacy-printer-study/README.md` and its machine-readable photo records.
The BX reference suggests PDF417-like escapes, but applying B7 substitutions
contradicted these measurements. The previous speculative substitutions and
pipe rejection have therefore been removed, together with the unused
`prepare_zpl_field` helper. The public byte encoder is unchanged.

Literal handling is a consistent implementation policy across Legacy qualities;
hardware confirmation currently covers ECC000/F6 on this firmware only. Other
qualities and firmware may require separately evidenced compatibility behavior.
Do not label this observation a universal interpretation of the Zebra manual.

Rust API note: `BarcodeDatamatrixWithData`, `RecalledFieldData` and
`RecalledField` now carry optional `data_bytes`. Existing struct-literal callers
must initialize it. For parsed Legacy fields it is authoritative; callers
replacing `data` must update/clear `data_bytes` too. Manually constructed ASCII
fields can use `None`; non-ASCII fields require explicit bytes. EPL continues
to use its existing ECC 200 text path.

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

## Unresolved long numeric record length

Zebra's Maximum Field Sizes table lists 596 for ECC 000, format ID 1. This is
also present in FCD Table G.1 (printed page 68, PDF page 76), with 560 for a
47x47 symbol and 596 for 49x49. Yet FCD 6.5.3 (printed page 24, PDF page 32)
defines a nine-bit field containing the number of user characters. Both pages
were checked visually: this is not merely a text-extraction discrepancy.

The current encoder deliberately refuses lengths above 511 until the extended
length convention is understood. That restriction does not implement Zebra's
documented maximum. Storing only nine low-order bits (512 -> 0, 596 -> 84) is
one observed approach in other code, but is not proven correct by this table.
No printer or decoder verification of the extended 512..596 convention has
been established. The separate 500/501 printer observation below does not
resolve that convention.

A regression checks all 30 cells of the Zebra maximum-field table: the other
29 cells must encode at their limit and reject one additional character. The
ECC 000 / ID 1 cell explicitly records the unresolved limitation instead of
pretending 511 is the documented maximum. Recovered module bits and CRC/payload
checks would be needed to establish an extended-length convention; table
capacity alone does not define the transmitted length field.

### ZD421 observation: 500 prints, 501 gives INVALID-L

On 2026-09-11 the contributor reported that a ZD421 prints 500 repeated ASCII
digits `1`, while 501 produces `INVALID - L` with validation enabled. The
500-digit control succeeded with both explicit quality 0 and omitted quality.
Settings: format 1, automatic dimensions, `^CI27`, `^CVY`, module size 3,
300 dpi, 600x300 dots (50x25 mm media), `~SD15`, `^MD0`, `^PR2`.
Firmware V93.21.17Z was recorded earlier, not queried again for this observation.
The supplied control ZPL was counted: its left field has exactly 500 digits;
the right field has 512 but no explicit physical result was reported for it.
The initial assistant socket attempt timed out; subsequent physical outcomes
were supplied by the contributor. These are user reports, not newly decoded
matrix fixtures or claims of scanner verification.

The test files and original report are preserved at the immutable Toolkit
commit [080a0f2](https://github.com/Streetblock/zpl-toolkit/tree/080a0f2/examples/bx-printer-study):
`05-legacy-length-endpoints.json`, `05-legacy-length-endpoints-300dpi.zpl`,
and `05-user-500-control.txt`.

This establishes the adjacent 500/501 boundary for the tested device and
settings. It does not establish a universal 500-character Legacy limit.
501 is representable in nine bits, so that rejection cannot be explained by
nine-bit overflow. The generic Rust encoder still accepts 501..511 when they
fit: this is a known difference from the tested printer. No unverified global
limit is introduced. A device-specific validation policy and `^CV` diagnostic
rendering remain separate follow-up work.

### Follow-up: explicit 49x49 does not bypass the boundary

The contributor printed the six-label AUTO/fixed49 packet from commit
`af4182e` and supplied a photo of N500/N501 on 2026-09-11. Both N500 fields
contain symbols; both N501 fields show INVALID-L. The accompanying report
states that all tested longer fields (511, 512, 596, 597) also fail in both
columns. Those latter rows are reported, not visible in the supplied photo.
The N500 symbols differ visibly; exact module counts and decoded content have
not been recovered. Model/firmware are inherited from the preceding experiment.

The fixed-size request therefore does not lift the observed boundary. This
closes the size-selection versus field-length printer probe for the tested
settings, but provides no extended record to resolve the nine-bit/596 conflict.
Keep the generic encoder policy unchanged and do not infer a universal cap.
See [the results and retained photo](../examples/legacy-printer-study/LENGTHS.md).
