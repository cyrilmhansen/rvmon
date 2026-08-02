# MiniBASIC-RV assembly port diary

## 2026-08-02 — capacity, long names, and the first full regression pass

- Extended the target-side numbered-line store to 256 records (`10..2560`) and
  separated the monitor's editable source capacity from its larger assembly
  scratch capacity. The boundary tests for 2560 BASIC lines and 8192 assembly
  instructions passed under QEMU.
- Adapted the Hammurabi demonstration to descriptive long variable names. The
  first failures were not resolver corruption: names beginning with dispatcher
  keywords (`G`, `E`, `F`, and the historical short-variable paths such as `Y`)
  were being classified before generic assignment. The tutorial now uses
  `CORNSTOCK` and `REGNALYEAR`, and the complete five-year run reaches the
  expected binary64 result.
- Generated the optimized Rust payload assembly and documented it as a
  semantic/porting oracle. Rust remains a reference path; BASIC parsing and
  arithmetic are still executed by the target assembly payload.
- The first broad QEMU matrix exposed a regression in `array-table`. Moving the
  line-length pointer from offset 584 to 600 collided with the short-array
  descriptor for variable `C`. The length pointer was moved again to the free
  offset 800. The array-table test then passed.
- The array investigation also showed that expression evaluation can reuse
  `x24`, which normally carries the current line record. The payload now saves
  and restores that context around short-array paths and before numeric print
  result handling.
- The same broad pass then exposed a second map collision: offset 800 was the
  short string-array descriptor table. The line-length pointer now lives at
  offset 480; numeric descriptors remain at 584 and string descriptors at
  800. Both string-array bounds and short string-array 2D tests pass again.
- The first matrix invocation also attempted two existing non-executable test
  scripts directly; those failures were harness permissions, not runtime
  failures. Re-running the affected string, GOSUB, and DATA/READ tests through
  `bash` produced 8/8 passes. The remaining global matrix is still required.
- After relocating the length table to 480, the previously failing long string
  array path also passes. The broad matrix had reached 53/54 before this final
  targeted rerun; the only failure was from the now-fixed descriptor collision.
- A parallel full run through `bash` completed with 54/54
  `test-guest-runtime-asm-repl-*.sh` tests passing under QEMU. This includes
  the previously sensitive short/long string arrays, Hammurabi, GOSUB,
  DATA/READ, interrupts, long identifiers, and calculated indices.
- The final verification also passed the 2560-line BASIC boundary test, the
  host source-capacity test, the `basic` command assembly-payload test, a
  target `cargo check`, and `git diff --check`. The assembly payload changes
  are therefore ready for an isolated fine-grained commit; unrelated dirty
  worktree changes remain deliberately unstaged.
- After the payload commit, a focused QEMU rerun passed the long-name
  Hammurabi session and the long two-dimensional string-array case. The build
  still emits known compressed-instruction overlap warnings from the generated
  decoder; these are pre-existing diagnostics and did not fail the tests.
- The assembly guide was audited and its opening status was corrected: its
  small modular examples are now explicitly separated from the much larger
  integrated payload, and old chronological paragraphs are labelled as such.
  I deliberately did not stage the guide yet because it already contains
  unrelated worktree edits from the user.
- I added a first target-side `WHILE`/`WEND` implementation with an eight-level
  stack and isolated QEMU tests. The first assembly attempt exposed two real
  integration issues: the existing 512-label scratch table was too small for
  the expanded payload, and newly added conditional branches exceeded the
  RISC-V branch displacement range. The symbol capacity is now 1024 and the
  long branches use inverted local branches plus `jal`.
- An initial branch-range rewrite around array-name validation accidentally
  inverted its semantic condition. The isolated short-array test caught this
  immediately: valid `A(...)` accesses produced `ERR`. I corrected the
  rewrite, reran the test successfully, and am keeping the incident here as a
  reminder that range-preserving transformations require behavioral tests.
- The WHILE/WEND tranche was committed as `ad917cb`. The workspace Rust test
  suite passed, and the focused QEMU matrix passed sequentially for arrays,
  strings, GOSUB/RETURN, DATA/READ, Hammurabi, WHILE/WEND, and orphan-WEND
  diagnostics. One existing long-string-array harness is timing-sensitive
  under repeated runs; an isolated traced run passes, so this remains a test
  infrastructure issue to harden separately rather than a hidden runtime
  failure.
- The next parity tranche adds target-side `REPEAT/UNTIL`, including nested
  loops, numeric truth values, binary64 comparisons, and an orphan-UNTIL
  diagnostic. The first version deliberately exposed two useful failures:
  comparator text was initially ignored after the left-hand expression, and
  the larger payload pushed two existing branches beyond the RISC-V ±4 KiB
  conditional-branch range. The focused QEMU test caught the first behavior;
  a static branch-distance audit caught the second. Both were corrected before
  the repeat tests were accepted.
- The post-`REPEAT/UNTIL` sequential guest matrix reached 57/58 on its first
  pass. The sole miss was `long-string-array-2d`; it passed on isolated runs
  1 and 3 and reproduced the known prompt-synchronization flake on run 2.
  This is evidence for a harness timing defect, not a runtime regression. The
  full matrix still exercises the new loop paths alongside the existing array,
  string, input, control-flow, and Hammurabi cases.
- The local ASM-One v1.48 Markdown reference is present under
  `docs/dontcommit/` and is intentionally ignored by Git. I began comparing
  its CLI vocabulary (`@D`, `@H`, `@N`, `G`, `K`, `M`, `N`, `S`, `F`, `C`, `Q`)
  with the modern monitor command surface; it will be used as a compatibility
  reference, not copied as a product specification.
- The intermittent matrix miss was isolated to the long string-array 2D test
  harness. Replacing `((condition)) && break` with an explicit `if` block under
  `set -e` made five consecutive QEMU runs pass. No runtime source changed in
  this hardening step.
