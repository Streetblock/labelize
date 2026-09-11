# Manual Legacy escape probes

Print `legacy-escapes-300dpi.zpl` for both labels, or `l01-300dpi.zpl` and
`l02-300dpi.zpl` individually. Exactly one copy of each label. These are native
printer jobs, not Labelize-rendered graphics or Labelary golden fixtures.

Media: 50x25 mm, 300 dpi, SD15, MD0, PR2. First text starts at 36 dots (about
3 mm) from the top. Each barcode has the same fixed 23x23 modules at 5 dots,
ECC000, format 6, CI27. CVY is enabled during probes and reset to CVN after them.

L01 compares `A\&B`, explicit FH CR/LF, and FH-created backslash-ampersand.
L02 compares doubled backslash, an FH-created single backslash, and double pipe.
Exact transmitted strings and hypotheses are in `manifest.json`. Parameter g
is omitted; `^FH#` selects the hexadecimal introducer only for its own field.
All data fits the chosen symbol under the candidate literal/escape meanings.

Record each code's presence, exact error text, and whether its matrix matches
the others on the same label. Photograph both labels straight-on. If a decoder
supports Legacy, preserve raw bytes: scanner line endings or displayed text
can hide the CR/LF distinction. An equal matrix proves equal encoded symbols
under these fixed settings, not by itself the intended decoded byte values.
Do not assume `||` means backslash before observing the result.

These first probes do not settle CI13 differences, unknown/trailing escapes,
single-pass behavior of overlapping sequences, or other ECC levels. Choose
follow-up probes based on the initial outcomes. Keep observations separate from
the generated manifest. See the measured outcome below.

Regenerate with `node examples/legacy-printer-study/generate.mjs` from the repo
root. The script checks field counts and horizontal bounds; it does not send
anything to a printer. Manifest hashes describe generated LF files; Git may
convert line endings on checkout.

References:
- [Zebra BX](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html)
- [Zebra B7 field rules](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-b7.html)

## ZD421 photo outcome, 2026-09-11

The compact layout (barcode top 78 dots, bottom 193 dots) printed all six
symbols completely. The photo and sampling details are identified in
`compact-photo-observation.json`; the sent job is in
`compact-print-observation.json`. Printer: ZD421-300dpi, V93.21.17Z.

| Position | Observed matching candidate bytes (hex) | Meaning |
| --- | --- | --- |
| L01 left | 41 5C 26 42 | Literal backslash and ampersand |
| L01 middle | 41 0D 0A 42 | FH-created CR/LF |
| L01 right | 41 5C 26 42 | FH-created literal backslash and ampersand |
| L02 left | 41 5C 5C 42 | Two literal backslashes |
| L02 middle | 41 5C 42 | One FH-created backslash |
| L02 right | 41 7C 7C 42 | Two literal pipes |

Each sampled 23x23 matrix matches its known-byte candidate at all 529 modules.
All six matches persist at common thresholds 130 through 150 in steps of 5.
Candidates use the Toolkit raw Legacy encoder: this is a matrix comparison,
not an independent scanner decode. No INVALID diagnostic appears.

For these CI27 settings, this contradicts the current Rust preprocessing:
backslash-ampersand must not be converted to CR/LF, doubled backslashes must
not collapse, and double pipe must not be rejected. Production code has not
yet been changed. Repeat the same fields with CI13 next to establish whether
the rules depend on character-set selection; do not generalize this outcome
to every firmware, character set or ECC level. Original manifest hypotheses
are retained as hypotheses, not measured results.
