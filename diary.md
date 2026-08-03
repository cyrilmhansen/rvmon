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
- I added `docs/ASMONE_PARITY.md` to make the comparison auditable without
  committing the v1.48 source itself. The comparison confirms that the host
  monitor already covers the essential assemble/disassemble/run/step/memory/
  breakpoint/watchpoint workflow with modern names. It also exposed a genuine
  BASIC design gap: TBXL's `POP`/`EXIT` semantics require a unified target
  control stack; clearing one of the current independent stacks would be
  incorrect, so I am not implementing that shortcut.
- I then implemented the first part of that design: a target-side unified
  control stack now records `FOR`, `GOSUB`, `WHILE`, and `REPEAT` frames, and
  `POP` removes the newest matching frame before a colon continuation. The
  first POP test exposed a stale `x21` statement pointer in the colon path;
  setting it explicitly to the character after `POP` fixed the failure. The
  resulting QEMU test covers both a GOSUB escape and a nested loop escape.
  `EXIT` remains intentionally unimplemented until its matching terminator
  scan can be made type-safe.
- At the user's request I added `docs/MINIBASIC_PARITY.md`. It records the
  observed TBXL-inspired surface, the 59 current QEMU assembly tests, the
  target-side proof for each implemented family, and the intentionally rejected
  Atari/DOS/graphics scope. The document explicitly labels `EXIT`, `DO/LOOP`,
  `IF/ELSE/ENDIF`, and `ON GOTO/GOSUB` as remaining decisions or work rather
  than implying full Turbo BASIC XL compatibility.
- I started the next parity tranche by implementing target-side `EXIT` for
  `FOR/NEXT`, `WHILE/WEND`, and `REPEAT/UNTIL`. The implementation validates the
  top typed frame, updates the specialized stack, pops the unified stack, and
  scans bounded source records for the matching terminator. The first QEMU
  attempt exposed a real RV branch-range regression: adding the handler made
  the existing conditional branch to `repeat_statement` exceed the 12-bit
  branch range. Replacing it with inverse-branch plus `jal` restored assembly;
  the dedicated EXIT test now reaches `EXIT-OK`. `DO/LOOP` remains explicitly
  deferred, and the parity document now reports 60 assembly QEMU tests.
- I extended the unified target stack with kind 5 for unconditional `DO/LOOP`.
  `DO` recognizes re-entry at the same source line without pushing a duplicate
  frame, `LOOP` validates and returns to that line, and `EXIT`/`POP` can remove
  the frame. Conditional DO/LOOP spellings remain rejected deliberately. The
  new QEMU test loops through an `IF` back-edge, exits the loop, and reaches
  `DO-OK`; the parity matrix now reports 61 assembly QEMU tests.
- I implemented `ON expression GOTO/GOSUB` with target-side 1-based selection
  over comma-separated line numbers. The normal QEMU test exercises selection
  of the second GOTO target and a GOSUB/RETURN target; a separate test rejects
  selector zero. Debugging exposed two low-level issues: the new handler first
  collided with a GOSUB stack slot, and RETURN incorrectly reused the unified
  stack depth when indexing the specialized return stack. Moving the scratch
  word and preserving the specialized depth fixed both. The parity matrix now
  reports 63 assembly QEMU tests.
- I implemented target-side structured `IF expression THEN` blocks with
  `ELSE` and `ENDIF`. The first QEMU run exposed a register-lifetime bug: the
  target expression evaluator reuses `x26`, which is also the current source
  line index. Restoring that index from the dispatch scratch before block
  control fixes both true and false branches. The normal and orphan-terminator
  QEMU tests now pass; the parity matrix reports 65 assembly QEMU tests.
- I added target-side numeric functions `TRUNC`, `FRAC`, and `MOD`. The first
  implementation exposed two register-lifetime hazards in the recursive
  evaluator: inner calls clobbered the outer `x30`/`x31` return state, and
  shared scratch slots broke `MOD(TRUNC(...),5)`. Saving the parser return
  registers and assigning distinct scratch ranges fixed both. QEMU now proves
  positive, negative, nested, and division-by-zero cases; `FRAC(-3.9)` is
  observed as the deterministic fixed-format `-0.899999`. The parity matrix
  reports 67 assembly QEMU tests.
- I added deterministic target-side `RND` and `RND()` using a 32-bit LCG
  (`1664525`, `1013904223`), with seed `1` at load and after `NEW`. The first
  implementation exposed the signed/unsigned conversion issue when a 32-bit
  state crossed bit 31; converting the low 31 bits and adding `2^31` when the
  high bit is set fixes the full `[0,1)` range. QEMU proves the first four
  sequence values and rejects `RND(1)`; the parity matrix now reports 69 tests.
- I revised `docs/MINIBASIC_PARITY.md` into an auditable parity register. The
  previous version had become stale (it reported 69 scripts while the current
  repository contains 70) and mixed historical TBXL parity with MiniBASIC
  extensions. The new version separates proven target-side behavior, partial
  semantics, planned work and deliberately rejected Atari scope, and records
  the exact QEMU evidence families and remaining roadmap.
- I added target-side `ABS`, `SGN` and `INT` to the assembly expression parser.
  `ABS` masks only the IEEE sign bit, `SGN` uses ordered D comparisons, and
  `INT` adjusts a signed round-toward-zero conversion for negative fractional
  values. The guest assembler rejected the convenient `fmv.d` spelling, so the
  zero result path uses the explicit `fmv.x.d`/`fmv.d.x` pair. QEMU proves
  positive, negative, zero, nested and syntax-error cases; the assembly test
  inventory now contains 72 scripts.
- I added target-side `LEN` for short/long string variables and string-array
  elements. The first functional test exposed that the `PRINT` pre-scanner
  classified any expression containing `$` as a string before reaching the
  numeric evaluator; recognizing the exact `LEN(` prefix fixes that dispatch
  boundary. A second test initially used invalid line number 45, which the
  MiniBASIC store correctly rejected; changing it to the required multiple of
  ten produced stable QEMU coverage for scalar, long-name, array and composed
  expressions. The documented assembly inventory now contains 74 scripts.
- I started the string-returning function tranche with `PRINT LEFT$(TEXT$,n)`.
  The first attempt exposed both a branch-range limit after growing the central
  dispatcher and RV64 sign-extension of the absolute `0x82060000` scratch
  address. Moving the function body near the end of the payload, replacing
  distant conditional branches with local trampolines, and zero-extending the
  address fixed both faults. QEMU now proves clamping, zero length and invalid
  count/arity cases; `LEN` remains green after the change. The inventory reaches
  76 scripts, while assignment and `RIGHT$/MID$` remain deliberately deferred.
- I added `PRINT RIGHT$(TEXT$,n)` using the same bounded target buffer and
  source-variable ABI as `LEFT$`, but copying from `source_length-n` so the
  suffix is selected without host intervention. QEMU proves suffix, clamp,
  zero-length and error paths, and the `LEFT$` regression remains green. The
  audit inventory reaches 78 scripts; `MID$` and string-valued assignments are
  still the next parity boundary.
- I added `PRINT MID$(TEXT$,start,n)` with 1-based positions, target-side
  clamping to the remaining source and explicit rejection of zero/negative
  starts, negative lengths and arity errors. Growing the dispatcher once more
  pushed an existing `read_find_line` conditional branch out of range; the
  inverse-branch plus `jal` form fixed it without changing runtime semantics.
  QEMU proves prefix, middle, suffix, empty-out-of-range and error cases; the
  audit inventory reaches 80 scripts. String-valued assignment remains the
  next architectural boundary.
- I updated the MiniBASIC/Turbo BASIC XL parity register after the first string-valued assignment tranche. The assembly payload now proves `LET destination$=LEFT$(source$,n)` entirely in the guest, including bounded counts, clamping, safe self-assignment through target scratch memory and clean diagnostics for invalid forms. The two new QEMU scenarios pass, bringing the reproducible assembly-test inventory to 82. The parity document now records this as PARTIEL rather than claiming general string-expression parity; `RIGHT$`, `MID$`, arrays, literals and general string expressions on the RHS remain explicit follow-up work.
- I extended the target-side string assignment dispatcher to `RIGHT$` and `MID$`. Both reuse the same bounded binary-copy discipline as `LEFT$`, including a target scratch buffer for self-assignment; `MID$` retains its 1-based start and empty-out-of-range semantics. Nominal, self-assignment, malformed/bounds and regression QEMU scenarios pass, bringing the assembly REPL inventory to 84 scripts. The parity register now treats all three scalar slice assignments as PARTIEL and keeps array/literal/general string RHS work explicit.
- I generalized the string-slice source resolver so `LEFT$`, `RIGHT$` and `MID$` can consume short and long string-array elements in the guest, while keeping assignment destinations scalar and bounded. A QEMU scenario proves short-array and long-array sources plus the existing scalar regressions; the reproducible assembly inventory reaches 85 scripts. Array destinations, literals and general string expressions remain the next explicit parity boundary.
- I connected the slice assignment dispatcher to short and long string-array destinations as well as scalar destinations. The resolved destination pointer is captured before RHS evaluation, so `LET A$(i)=LEFT$(...)`, `RIGHT$` and `MID$` use the same target scratch path and reject invalid counts before mutation. Nominal and error QEMU tests pass; the reproducible assembly inventory reaches 87 scripts. Literal RHS and general string expressions are now the next string-parity boundary.
- I added target-side literal sources for `LEFT$`, `RIGHT$` and `MID$` in both `PRINT` and assignment paths. Literal bytes are copied into a separate target buffer at `0x82060100`, leaving the result scratch at `0x82060000` intact for overlapping/self-assignment cases. Nominal and malformed/bounds QEMU scenarios pass; the assembly inventory reaches 89 scripts. General string expressions, concatenation and intrinsic composition remain the next boundary.
- I added target-side string concatenation for assignments: literals, scalar variables and short/long string-array elements can be combined with `+` into a bounded 120-byte target buffer, then copied atomically to scalar or array destinations. Invalid terms, missing operands and capacity overflow return through the normal guest diagnostic path. Nominal and error QEMU scenarios pass; the assembly inventory reaches 91 scripts. Slice functions inside concatenations and numeric-to-string conversion remain intentionally unsupported.
- I extended concatenation terms to `LEFT$`, `RIGHT$` and `MID$`, with exact function-name recognition so ordinary identifiers are not silently treated as intrinsics. The slice term uses a third target buffer (`0x82060300`) before the concatenation buffer, preserving the no-partial-write invariant. The nominal and error concatenation scenarios now prove prefix, suffix, middle and bound failures; the inventory remains 91 scripts.
- I tightened the assignment dispatcher to recognize the complete `LEFT$`, `RIGHT$` and `MID$` spellings before selecting the slice handlers. This keeps long identifiers such as `LONGTEXT$` on the ordinary string-source/concatenation path instead of misclassifying them by their first letter. The concat regression now covers long-name copy and append cases in the guest.
- I added target-side `ASC` and `CHR$`: `ASC` reads the first byte of a scalar, array or literal source and returns binary64, while `CHR$` converts a bounded integer to a one-byte string term for assignment and concatenation. Empty sources, missing arguments, negative values, fractions and values above 255 are rejected without partial writes. The existing concat nominal/error scenarios cover both operations and the long-name regressions.
- I completed the `CHR$` surface by adding exact `PRINT CHR$(expression)` dispatch. It now emits the target-produced byte through the guest console ABI and returns a newline, while negative, fractional, above-255 and missing arguments use the ordinary target diagnostic path. The same QEMU nominal/error scenarios cover assignment, concatenation, `ASC` and direct printing.
- I added `VAL(string-source)` to the target numeric evaluator. The guest resolves a literal, scalar or string-array source, copies its bytes into the target input buffer, evaluates them with the existing binary64 parser, and rejects empty/non-numeric/trailing input without consulting the host. The concat/character QEMU scenarios now also prove `12.5`, a variable-held `3.25`, and four invalid forms.
