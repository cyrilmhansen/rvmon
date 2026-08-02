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
