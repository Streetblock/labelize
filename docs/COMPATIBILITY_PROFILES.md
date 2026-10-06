# Compatibility profiles: integration design

`integration/render-profiles` combines PR [#48](https://github.com/GOODBOY008/labelize/pull/48)
and [#53](https://github.com/GOODBOY008/labelize/pull/53), then introduces the explicit
QR character modes from [#52](https://github.com/GOODBOY008/labelize/pull/52) behind
an opt-in profile. The fork's `main` continues to mirror upstream. The two open
fix PRs keep their own branches; this integration branch is a separate place to
review the combined behavior and profile design.

## Three independent choices

| Axis | Initial implementation | Later extension |
| --- | --- | --- |
| Command interpretation | `labelary` (default), `zebra-experimental` | Model/firmware-specific policies backed by printer evidence |
| Presentation/output | Existing raster PNG and raster embedded in PDF; existing antialias option | SVG and hybrid/vector PDF |
| Inspection | Existing parsed elements | Source-linked bounds, origins, dot grid and diagnostics as an optional overlay |

Debug is an inspection layer that can be used with either compatibility profile
and either presentation backend. A vector backend also needs a compatibility
profile. These choices must not become four mutually exclusive modes.

## Implemented profile policies

| Behavior | `labelary` | `zebra-experimental` |
| --- | --- | --- |
| ZPL QR automatic input (`QA,`, etc.) | Existing automatic segmentation | Same |
| Manual Numeric (`QM,N…`) | Existing automatic segmentation | One Numeric segment; ASCII digits required |
| Manual Alphanumeric (`QM,A…`) | Existing automatic segmentation | One Alphanumeric segment; QR's 45-character alphabet required |
| Manual Byte (`QM,Bnnnn…`) | Existing automatic segmentation after prefix/length parsing | One Byte segment after the same parsing |
| Manual Kanji (`QM,K…`) | Existing automatic encoding of parsed content | Explicit unsupported error; no automatic fallback |
| QR mask selection | PR53 policy | Same PR53 policy |
| DataMatrix dimensions, ratio and row-only capacity | PR48 policy | Same PR48 policy |
| Empty QR fields, fonts, positioning, mirroring, rasterization | Existing behavior | Same |
| EPL | Supported as before | Rejected by CLI/HTTP |

The experimental profile is deliberately narrow. It does not yet emulate a
complete Zebra printer. Byte input currently uses the Rust string's UTF-8 bytes;
printer code pages, raw 8-bit field data, ECI, Shift-JIS/Kanji and mixed manual
segments need separate work. The existing field parser and its lenient length
handling remain shared. In particular, this profile does not introduce strict
validation of every malformed ZPL prefix.

`LabelInfo` does not retain the source language. Library callers must apply
`ZebraExperimental` only to labels parsed from ZPL. The CLI and HTTP API enforce
this because the EPL parser synthesizes QR Byte prefixes even for ordinary EPL
data, which would otherwise acquire unintended manual-byte semantics.

The authoritative mode definition is Zebra's [^BQ command reference](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bq.html).
Unsupported or invalid explicit encodings return an error rather than producing
a symbol in another mode. This is an API error policy; printer `^CV` diagnostic
labels are not emulated by this change.

## API and selection

`Renderer` remains a unit struct and `DrawerOptions` keeps its existing fields.
The old `draw_label_as_png` entry point delegates to `Labelary`. Callers opt in
using an additive method. The library stores no mutable profile state; a
request override never changes the configured server default.

```rust
use labelize::{CompatibilityProfile, DrawerOptions, Renderer, ZplParser};

let zpl = b"^XA^FO10,10^BQN,2,2^FDQM,B002012345678901234567890^FS^XZ";
let labels = ZplParser::new().parse(zpl).unwrap();
let mut png = Vec::new();
Renderer::new().draw_label_as_png_with_profile(
    &labels[0],
    &mut png,
    DrawerOptions::default(),
    CompatibilityProfile::ZebraExperimental,
).unwrap();
```

CLI: `labelize convert label.zpl --profile zebra-experimental`.
HTTP: `POST /convert?profile=zebra-experimental` with `application/zpl` input.
Server: `labelize serve --default-profile zebra-experimental` sets a self-hosted
instance's default. Precedence is explicit request profile, then server startup
default, then the built-in `labelary` default. Existing HTTP callers and ZPL
bytes remain unchanged when the server starts without the new option. Profile
configuration stays outside ZPL; no renderer-specific ZPL command is added.

Both profiles apply to PNG and the existing raster PDF. Unknown names are rejected.
An EPL request inheriting the Zebra server default gets a 400; it can explicitly
select `profile=labelary`. WASM and Android bindings continue using
their existing default entry points; profile selection in those bindings is a
later additive API change.

## Evidence and regression contract

| Control | Observed result | Regression check |
| --- | --- | --- |
| PR48 standard rectangles at 8/12/16 rows | Row-only selection stays at the smaller standard width; capacity overflow yields no field. An explicitly wider rectangle fits. Labelary controls and user printer observations agree. | Independent Labelary fixture at 0% tolerance; byte and special-codeword constraint tests |
| PR53 five EC-H Byte controls | Corrected mask matches the complete Labelary matrices without changing data codewords. | Independent reference matrices, decoder checks and 0% golden tolerance |
| Q01, `QM,B002012345678901234567890` | Labelary uses Numeric/V1 (21 modules). Explicit Byte uses V2 (25 modules); the user confirmed that this matrix matches the native printer on the comparison print. | Compare full recorded matrices in `testdata/qr-profiles/`, then decode both profiles independently and check mode, size, EC-Q and identical content |
| S01, `HM,B0007abcabaa` | Labelary's mask and the independently decoded native-printer mask differ, even with identical data codewords. | Keep PR53 unchanged in both profiles; record the printer-mask question as unresolved |

The Q01 printer match is a user-confirmed visual observation, not an automated
full-matrix comparison against a committed printer scan. The tests establish
the explicit encoding and preserve the Labelary default. They do not claim
that all Zebra matrices or dots match. Golden tests continue to use the default
Labelary profile; printer-oriented controls form a separate evidence set.

Profile regression tests also cover automatic/empty fields, invalid explicit
data, reverse-print dispatch, per-call isolation, the five PR53 matrices in
both profiles, shared DataMatrix geometry, and PNG/PDF/HTTP selection.

## Next architecture steps

1. Add individually measured Zebra policies, starting with QR mask selection.
   Record ZPL bytes, printer model, firmware, resolution and module matrices;
   distinguish verified behavior from hypotheses. Do not infer a general
   Zebra policy from one printer or one sample.
2. Introduce a resolved display list between parsed elements and output. Store
   source spans, physical dot coordinates, resolved bounds and barcode module
   grids. Resolve compatibility decisions once before raster/vector backends.
3. Build the inspector over that display list: toggle origins, bounds and grid
   without changing printable output. Unsupported-command and encoding
   diagnostics should link to their source.
4. Add SVG/hybrid PDF: vector primitives for boxes, circles, ellipses and lines;
   resolved font outlines where appropriate; embedded images for raster fields.
   Barcode module grids keep their resolved size and placement. Existing raster
   output remains available for dot comparisons.

Steps 2–4 are design work only in this branch. No vector backend, debug overlay,
printer-specific font rasterizer or new global parser mode is introduced yet.

## Separate virtual-printer project (idea only)

Raw TCP/TLS ingestion is deferred and is not part of the proposed Labelize
profile work. A separate project could implement a virtual Zebra printer and
use Labelize's Rust library, a compatibility profile, or reusable components
as its rendering engine. Zebra's [SDK connection reference](https://techdocs.zebra.com/link-os/latest/pc/content/v2155569/com/zebra/sdk/comm/connectionbuilder)
uses TCP **9100** and TLS **9143** as defaults; TLS here is raw encrypted printer
traffic, distinct from HTTPS.

The benefit would be capturing ZPL from applications that already send directly
to printer sockets. However, merely accepting a TCP stream does not make a
drop-in virtual printer: clients may query status, download resources, keep
connections open, and send several labels with persistent settings. There is
also no HTTP response envelope in which to return PNG/PDF. An initial adapter
could save submitted jobs/results for retrieval over HTTP or an inspector; its
contract should state which status/control commands it supports.

That project would need to define streaming job boundaries (including binary
graphics, split commands and multiple labels), resource/state lifetime, output
delivery, bounded input/idle handling and TLS certificate configuration. It
could reuse the same parse/resolve/render pipeline while owning the emulated
device state and client protocol. The profile engine alone must not be
advertised as a drop-in printer emulator. No TCP or TLS listener is implemented
or scheduled as part of this branch.
