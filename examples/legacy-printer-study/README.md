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
follow-up probes based on the initial outcomes. No new hardware outcomes have
yet been recorded here. Keep observations separate from the generated manifest.

Regenerate with `node examples/legacy-printer-study/generate.mjs` from the repo
root. The script checks field counts and horizontal bounds; it does not send
anything to a printer. Manifest hashes describe generated LF files; Git may
convert line endings on checkout.

References:
- [Zebra BX](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bx.html)
- [Zebra B7 field rules](https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-b7.html)
