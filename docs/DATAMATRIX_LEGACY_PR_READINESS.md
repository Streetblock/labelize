# Legacy PR readiness

Checkpoint: 2026-09-11. Branch `feat/datamatrix-legacy`. A combined Legacy PR
is intended; it has not yet been submitted. This is a work plan, not a claim
that all checks below have already passed on the eventual submission commit.

## Completed

- All five Legacy qualities, corrected generated placement and randomization,
  byte-preserving ZPL field handling, and documented initial escapes exist.
- The ZD421 500/501 boundary is recorded in `DATAMATRIX_LEGACY.md`, including
  evidence limits and the deliberate difference from the generic encoder.
- Two small native printer jobs for CR/LF, FH order, backslash and double pipe
  are prepared in `examples/legacy-printer-study`. Physical outcomes pending.
- Read-only three-way merge inspection against PR #55 commit `c410a33` using
  base `c5ae397` finds textual conflicts in `src/barcodes/mod.rs` and
  `src/drawers/renderer.rs`. No merge was performed by this inspection.

## Remaining, in order

1. Record L01/L02 device outcomes. Resolve double-pipe behavior based on
   evidence; select any needed follow-ups for CI13, overlapping escapes and
   other ECC levels. Turn supported observations into targeted regression tests.
   Keep raw encoder bytes separate from ZPL preprocessing.
2. Reconcile with upstream PR #54 (quality handling) and PR #55 (ECC200 field
   escapes), choosing the final base after their current upstream state is
   checked. The Legacy branch already includes the #54 fix. Resolve the two
   known conflicts, and review the jointly modified parser, EPL path and
   DataMatrix element even where Git reports no textual conflict. Verify
   ECC200 escapes never run through Legacy preprocessing and vice versa.
3. Review public struct changes: `BarcodeDatamatrixWithData`, `RecalledFieldData`
   and `RecalledField` gained optional `data_bytes`. Document struct-literal
   migration and the requirement to update/clear bytes when replacing display
   text. Check stored-format recall and independent manually constructed fields.
4. Keep `^CV` diagnostic rendering and any device-specific 500-character policy
   as explicit follow-ups, outside the combined encoder PR. The generic
   511-character guard remains a documented unresolved implementation limit;
   neither 500 nor wrapped nine-bit lengths become a universal rule.
5. After final implementation/base changes, run formatting, clippy, affected
   unit/integration tests and the required render diff and golden checks from
   AGENTS.md. Review and commit resulting render artifacts. Earlier validation
   recorded in commit `24b76ac` is not a substitute for final-commit checks.
6. Rewrite the submission description around final behavior, evidence and known
   limitations; push the Legacy branch and open the combined PR. Distinguish
   same-source JS regression fixtures, norm examples and printer observations.

The historical FCD table itself includes 596 for ECC000/format1; its conflict
with the nine-bit character-count description is not uniquely a Zebra manual
issue. Do not claim a resolved extended-length convention or claim all 596
digits are supported.

Related upstream work:
- https://github.com/GOODBOY008/labelize/pull/54
- https://github.com/GOODBOY008/labelize/pull/55
