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
- I added target-side `INSTR(haystack,needle)`. The guest resolves both sources, scans byte-by-byte in target RAM, returns a 1-based binary64 position, `0` for no match and `1` for an empty needle, while preserving the existing diagnostic path for missing or non-string operands. The QEMU scenario proves prefix/middle matches, no match, empty needle and malformed calls.
- I extracted a target-side fixed-six-decimal formatter for `STR$(expression)`. It returns a pointer/length pair from a dedicated buffer, preserves signs and fractional digits, and composes with literals, variables and slice terms without involving the host. The QEMU nominal/error scenario proves positive and negative values plus a missing argument; direct `PRINT STR$` remains deliberately deferred.
- I completed the `STR$` surface with exact `PRINT STR$(expression)` dispatch. It now emits the same target-produced fixed-six-decimal buffer used by assignment and concatenation, with the same missing-argument diagnostics. The QEMU scenario covers all three use sites.
- I audited the supposedly partial structured IF implementation with a dedicated nested program. QEMU proves both a true outer block containing a false inner block and a false outer block whose nested branch must be skipped before entering the outer ELSE. The existing depth counter and unified control stack handle this within the eight-level limit; the parity register now marks nested `IF/ELSE/ENDIF` VERT and the inventory reaches 92 scripts.
- I refreshed `docs/MINIBASIC_PARITY.md` as the explicit TBXL/MiniBASIC tracking register. The audit now distinguishes preserved interaction, RV-specific modernization, proven partial families, deferred historical functions, and rejected Atari facilities. I also removed stale roadmap wording that treated the already implemented string slices as future work and replaced the obsolete test-count description with the current target-side evidence scope.
- I added target-side `SQR(expression)` using the RISC-V `fsqrt.d` instruction, including domain rejection for negative operands and deterministic guest-produced output. The integrated assembler now accepts `fsqrt.d`; a latent overlong payload label was shortened, and local jump trampolines keep conditional branches in range after the payload grew. QEMU proves `SQR(9)`, `SQR(2)`, `SQR(0)`, nested `SQR(ABS(-16))`, and two negative-domain diagnostics; the host never evaluates the result. The language test plan and parity register now record SQR as target-side verified, while SIN/COS/TAN/ATN/LOG/EXP remain deferred.
- I added target-side `SIN(expression)` and `COS(expression)`. The guest performs radians reduction with binary64 constants, evaluates a bounded Taylor/Horner polynomial using D instructions, handles canonical `0`, `±pi/2` and `pi` results exactly, and keeps separate save frames for nested SIN/COS calls. The constants were deliberately moved to `0x82010000`, outside the payload scratch/workspace and array regions; an initial collision with `x8+1032..1048` was caught by the debugger because runtime text overwrote coefficients. QEMU now proves general values, canonical points and nested calls (`SIN(COS(0))`), and the SQR regression remains green. The parity register promotes SIN/COS to VERT; TAN, ATN, LOG and EXP remain deferred.
- I added target-side `TAN(expression)` by evaluating the guest-produced sine and cosine through the shared math core, then dividing with `fdiv.d`. TAN has its own static frame so nested `TAN(COS(0))` does not overwrite SIN/COS state; a zero cosine is rejected before division. The first QEMU run exposed that `eval_expression` reuses `x29`, so the TAN frame pointer is rebuilt after evaluation; the corrected nominal and pole tests pass. The parity register now marks direct trigonometry VERT, leaving ATN, LOG and EXP as the next mathematical gap.
- I added target-side `LOG(expression)` and `EXP(expression)`. LOG decomposes normal positive binary64 values into exponent and mantissa and evaluates the atanh-style series in D; EXP reduces by `ln(2)`, evaluates a degree-10 Horner polynomial and reconstructs the binary64 exponent. Debugging caught two concrete issues before acceptance: an incorrect `ln(2)` address offset and a missing second constant term in the Horner form. QEMU now proves LOG/EXP nominal values, `EXP(LOG(10))`, and the `LOG(0)` diagnostic; V1 bounds the domain to normal positive LOG inputs and `[-708,708]` for EXP. ATN remains the next mathematical gap.
- I refreshed `docs/MINIBASIC_PARITY.md` as the explicit TBXL/MiniBASIC tracking register. It now separates green target-side evidence, documented partial compatibility, rejected Atari-specific scope, and work in progress. The ATN implementation and its two QEMU scenarios are recorded as EN COURS rather than being counted prematurely as accepted parity; the register now inventories 101 assembly-oriented QEMU scripts, of which 99 are already green evidence and two await validation.
- I completed and validated target-side `ATN(expression)`. The first QEMU run exposed a real context-restore bug: the saved `x29` value was incorrectly dereferenced as a pointer, causing a load access fault before `ATN(0)` could print. Restoring `x29` directly from the depth-specific static slot fixes both the top-level and nested frames. QEMU now passes `ATN(0)`, `ATN(±1)`, `ATN(0.5)`, `ATN(10)`, `ATN(ATN(1))`, and the incomplete-call diagnostic; the parity inventory reaches 101 scripts and ATN is promoted to VERT.
- I added target-side `DEL n` and `DEL n,m` after auditing the TBXL editor commands. The implementation parses and validates 10..2560 multiples of ten, deletes an inclusive range from the fixed line-length table, and rejects reversed ranges without partial writes. QEMU proves single-line deletion, range deletion, sorted `LIST`, execution of surviving lines only, and the invalid-range diagnostic. `RENUM` remains the next editor-parity decision; file/DOS commands remain intentionally outside the RV target scope.
- I recalculated the assembly REPL evidence inventory after adding the DEL scenario: 102 `test-guest-runtime-asm-repl*.sh` scripts are now present. The parity register keeps historical journal counts unchanged but reports 102 as the current reproducible audit total.
- I added target-side `RENUM new,old,step` in a deliberately partial but safe form. The fixed records are prevalidated, renumbered in place with `new >= old`, and simple programs preserve sorted listing and execution; programs containing `GOTO`, `GOSUB`, `THEN` or `ON` are rejected before any write because reference rewriting is not implemented yet. The first integration attempt exposed a dispatcher gap (`RENUM` was mistaken for a long variable beginning with R) and two old branches moved beyond the 4 KiB conditional range; both were corrected with explicit command routing and inverse-branch/jump trampolines. QEMU nominal and no-partial-write error scenarios pass, bringing the inventory to 104 scripts.
- I replaced the temporary RENUM reference rejection with a compact target-side alias: each renumbered record stores its previous line number at `record+8`, and `GOTO`, `GOSUB`, `IF THEN` and `ON GOTO` resolve either the current number or that alias. The new QEMU control-flow scenario passes, the validation-error scenario now uses `STEP=0`, and the full workspace test suite passes. This is deliberately limited to the last renumbering; permanent reference rewriting remains a documented parity decision.
- I audited a proposed target-side `PRINT` string-concatenation extension against the existing long-string regression. The short literal case was insufficient evidence: the long-variable path still returned a real guest error, so I removed the unvalidated experiment and recorded the surface as partial rather than promoting it in the parity register.
- I refreshed `docs/MINIBASIC_PARITY.md` as the durable TBXL/MiniBASIC parity register and reconciled its reproducible inventory to 106 QEMU assembly-REPL scripts. The register now clearly separates preserved interaction, RV-specific modernization, proven partial surfaces, deferred work and rejected Atari-specific scope; the current string-concatenation experiment remains explicitly unaccepted until its target-side QEMU scenario is green.
- À la demande de l’utilisateur, j’ai établi `docs/MEMORY_MAP.md` comme contrat de placement commun au moniteur et à MiniBASIC. L’audit de concaténation a révélé et corrigé plusieurs collisions concrètes : les scratchs binary64 `1024/1032/1040/1048` écrasaient le buffer d’entrée `x18=x8+1024`, puis les retours `2000`/`1800` entraient en conflit avec les cadres de découpe et numériques. Les scratchs numériques sont maintenant distincts après la pile de contrôle et les retours chaîne disposent de cellules globales dédiées ; le scénario QEMU minimal de concaténation est vert. Le test complet des chaînes est en cours de vérification et ne doit pas encore être déclaré vert sans son résultat réel.
- Le test complet `test-guest-runtime-asm-repl-string-concat.sh` n’est pas accepté : après les sorties de chaînes de base, il rencontre encore `ERR` dans la séquence `ASC/VAL/INSTR/STR$` et manque le résultat `72.000000`. Cette observation est conservée comme preuve négative ; la carte mémoire est désormais le contrat à consulter avant de modifier ces routines.
- J’ai commencé une passe de commentaires pédagogiques directement dans `examples/minibasic-asm/payload-repl.rv` : convention des registres, représentation des records, pipeline REPL→RUN→statement→expression, piles de contrôle, tableaux, chaînes, concaténation et fonctions mathématiques. Les commentaires renvoient désormais à `docs/MEMORY_MAP.md` afin que le source explique les invariants sans recopier une carte divergente. Le cas réduit `ASC(TEXT$)` reste en échec et sera traité comme défaut fonctionnel séparé, pas masqué par cette passe documentaire.
- J’ai poursuivi cette passe dans le source assembleur : le buffer d’entrée et les scratchs binary64 sont maintenant décrits avec leurs bornes réelles, et les algorithmes de `INSTR$`, `VAL`, `ASC`, de l’affectation chaîne et de la concaténation expliquent leurs couples adresse/longueur, leurs cadres de sauvegarde et leur invariant d’écriture atomique. Aucun comportement n’a été modifié ; l’échec réduit de `ASC(TEXT$)` reste explicitement ouvert.
- J’ai corrigé le dispatch `PRINT` afin que `ASC(...)`, `VAL(...)` et `INSTR$(...)` ne soient pas confondus avec une expression chaîne contenant un `$`. `ASC(TEXT$)` et `ASC(A$(0))` produisent maintenant bien 72 dans le scénario QEMU complet. Le diagnostic de `VAL("12.5")` a ensuite été instrumenté côté cible : la copie vers le buffer et le motif binary64 12,5 sont confirmés, mais le chemin `resolve_string_source` peut encore partir dans son scanner de variables au lieu de prendre le littéral cité ; le test complet reste donc non accepté. Les sondes temporaires ont été retirées et `scripts/tmp-debug-val.sh` supprimé. La parenthèse fermante de `VAL` est désormais sauvegardée après consommation, correction conservée mais à revalider après résolution du défaut de reconnaissance du littéral.
- Une mesure plus fine a confirmé que `resolve_string_source` reçoit bien `x21` sur le guillemet ASCII `0x22`. Une tentative de trampoline `bne`/`jal` à la place du `beq` direct a provoqué une faute de lecture dans le chemin non-littéral et a été retirée immédiatement. Le source revient donc à la dernière version connue sans cette régression ; `VAL("12.5")` reste ouvert et aucun test global n’est promu vert.
- Un micro-programme indépendant `lbu` depuis `0x82000000`, suivi de `addi x6,34` et `beq x5,x6`, passe avec `x5=x6=0x22` et atteint le label attendu. Le défaut n’est donc ni l’instruction `lbu`, ni le branchement RV64 générique. Les sondes ajoutées dans `resolve_string_source` et `resolve_string_literal_source` ont été supprimées après la mesure ; il reste à isoler le retour/cadre de la routine littérale dans le payload sans modifier le comportement accepté.
- À la demande de l’utilisateur, j’ai ajouté `stepidp N` au moniteur guest. La commande suit les PC réellement calculés, imprime après chaque pas les registres entiers et flottants bruts, `fcsr`, `fflags` et une fenêtre de 16 octets alignée sur `x2` avec comparaison avant/après. Le test QEMU dédié a d’abord révélé que le programme de test écrasait `x2` avec `addi x2,x1,2`, ce qui provoquait une faute lors du trap suivant ; le test corrigé conserve la pile. Il couvre maintenant trois pas, dont un `jal` et un `fadd.d`, et passe avec exactement trois rapports. La documentation du tutoriel et l’aide du moniteur ont été mises à jour.
- Une première séance instrumentée sur le payload MiniBASIC a confirmé que `stepidp` suit les cibles de saut et imprime le contexte complet à chaque arrêt. Le breakpoint posé sur `resolve_string_literal_done` a toutefois interrompu la boucle de commande avant l’injection effective du scénario `VAL`; cette trace est utile comme preuve d’outil, mais ne constitue pas encore un diagnostic du défaut `VAL`.
- La trace `stepidp` de `VAL("12.5")` a isolé une corruption du contexte dans `resume_user` : `t0`/`x5` était restauré puis réutilisé pour charger les CSR, et quittait donc le trap avec la valeur de `mstatus`. Après restauration, le branchement littéral de `resolve_string_source` pouvait être faux malgré `lbu x5` ayant bien lu `0x22`. J’ai ajouté une restauration finale de `t0` après le dernier `csrw mstatus`; le test `stepidp` contient désormais un `beq x5,x6` qui échouerait avec l’ancien défaut. La trace VAL confirme aussi le motif binary64 `0x4029000000000000` dans `f4`.
- Après restauration de `t0`, la trace a montré que `VAL` atteignait bien `f4=0x4029000000000000`, mais bouclait ensuite dans l’évaluateur externe. La cause était l’usage de `x31` comme retour de `eval_expression` imbriqué sans sauvegarde par `atom_val_function`. Le payload réserve désormais `x8+2432` à ce cadre et restaure `x31` avant le retour vers l’évaluateur appelant.
- La régression instrumentée a ensuite montré que `INSTR("HAMMURABI","MUR")` cherchait le second littéral dans lui-même : les deux résolutions réutilisaient le même buffer cible `0x82060200`. `atom_instr_function` copie maintenant le premier opérande vers `0x82060730`, zone réservée dans `docs/MEMORY_MAP.md`, avant de résoudre le second. Une première tentative a été rejetée par le dialecte à cause de `0x730`, puis corrigée en `1840` ; une seconde a révélé la sign-extension RV64 de `lui`, corrigée par la paire `slli/srli`. Le test QEMU complet est maintenant vert et produit la position correcte `4.000000` dans `HAMMURABI`.
- La remise au vert du scénario d’erreurs a révélé deux défauts de bord target-side : `VAL("")` entrait dans l’évaluateur vide et pouvait atteindre `run_stop`, tandis que `PRINT STR$()` traitait `)` comme la valeur numérique zéro. `atom_val_function` rejette maintenant une source de longueur nulle et `print_str_string` rejette explicitement une parenthèse fermante immédiate. Le test envoie chaque erreur en mode direct, attend le chargement UART réel du payload et vérifie 20 diagnostics ainsi qu’un `10 END` final ; QEMU passe.
- J’ai ajouté une passe de documentation directement dans `examples/minibasic-asm/payload-repl.rv`. Le source nomme maintenant ses variables logiques, décrit les records, tables, frames et piles statiques, et introduit les algorithmes REPL→RUN→dispatch→expression ainsi que les familles de résolution, tableaux, chaînes et formatage. Les commentaires restent après `assemble-program`, conformément au protocole du moniteur ; l’expérimentation de concaténation PRINT reste non acceptée et non incluse dans ce commit.
- J’ai terminé la première tranche de concaténation dans `PRINT`. Le défaut venait du branchement des chemins `LEFT$/RIGHT$/MID$` vers les wrappers d’affectation ; ils sauvegardent maintenant le début de l’expression et réutilisent le concaténateur target-side commun. QEMU passe `"RV "+TEXT$`, `TEXT$+"!"`, `LEFT$(TEXT$,4)+"!"` et `"A"+"B"+"C"` sans `ERR`. J’ai aussi corrigé le test arithmétique historique pour inspecter `x8+2408` (`0x82000968`), où `print_result` conserve réellement le motif binary64, plutôt que le buffer d’entrée réutilisé par `RUN`.
- J’ai étendu cette tranche aux fonctions chaîne `CHR$` et `STR$` dans `PRINT`. Le payload sauvegarde le début de l’expression puis repasse par le concaténateur target-side ; QEMU valide maintenant `CHR$(65)+"B"` et `STR$(12.5)+"!"` en plus des littéraux, variables et découpes. La matrice de parité distingue cette sortie validée de l’affectation concaténée encore partielle.
- La capacité `assemble-program` a été portée de 8192 à 9216 lignes pour absorber le payload assembleur MiniBASIC actuel (8339 lignes non commentées), sans modifier la capacité de l’éditeur persistant ni la pile M-mode. Le test QEMU de limite accepte exactement 9216 lignes et rejette explicitement la 9217e avec `GUEST-ASM-001`; la documentation ABI a été réalignée sur 1024 symboles.
- J’ai ajouté `stepidp [count]` aux consoles host et backend du moniteur. La commande suit le PC produit par chaque instruction, rend l’état complet des registres, `fcsr/frm/fflags` et la pile d’appels après chaque pas, et affiche un bloc de 16 octets autour d’un store observé. La borne de 256 pas, les cas zéro/hors limite et les écritures target-side sont couverts par des tests unitaires ; la syntaxe et la parité ASM-One sont documentées.
- L’audit QEMU a trouvé un défaut réel dans les tableaux courts : le dispatch de l’identifiant `C` partait d’abord vers le probe `COS`, puis son repli appelait toujours le résolveur de tableaux longs. `C(2)` provoquait donc un accès fautif alors que `B(1)` fonctionnait. Le probe teste maintenant `C(` avant `COS(` ; `test-guest-runtime-asm-repl-array-table.sh` passe et valide les deux tableaux ainsi que le calcul `16.000000`.
- La campagne a aussi débusqué un test d’affectation obsolète : il cherchait le résultat `PRINT X+3` dans le buffer d’entrée `x8+1024`, déjà réutilisé par `RUN`. Le scénario pointe désormais vers la cellule documentée `x8+2408` (`0x82000968`) ; `test-guest-runtime-asm-repl-assignment.sh` passe sans changement du payload.
- Le scénario `string-array.sh` semblait bloqué parce qu’il synchronise chaque ligne du payload assembleur avec une lecture du prompt ; l’injection complète prend environ une minute. Après une exécution sans délai artificiel trop court, le test passe réellement : tableaux chaîne courts et longs, index expression, `PRINT`, et programme numéroté produisent les sorties attendues. Aucun changement de comportement n’était nécessaire.
- J’ai étendu `LEN` au cas `LEN("littéral")`. Le payload réutilise `resolve_string_source`, sans nouvelle cellule mémoire ni délégation hôte ; le chemin variable/tableau saute explicitement le nouveau branchage littéral. Le scénario QEMU `string-len.sh` ajoute `LEN("RV64")` pour verrouiller ce cas.
- Le premier scénario de validation plaçait ce littéral en commande directe, ce qui déclenchait correctement le `run_stop` du payload avant les lignes suivantes. Je l’ai corrigé pour l’exécuter dans un programme numéroté ; le test QEMU complet passe avec les résultats 4, 5, 4, 3 et 9.
- J’ai complété les commentaires pédagogiques du payload assembleur autour du dispatch de `PRINT`, de l’ABI interne des chaînes, du buffer partagé des littéraux, du formatage binary64 et des continuations de `print_result`. La passe est documentaire uniquement ; le test QEMU `string-len.sh` reste vert.
- Le scénario complet de concaténation, rejoué avec un délai d’injection suffisant, a révélé un vrai défaut target-side : `RIGHT$(...)+...` dans une affectation sautait vers `print_slice_concat` et provoquait un fault à `resolve_string_concat_term`. Le branchement vise maintenant `string_right_assign_concat`; le nouveau test QEMU dédié passe avec `RABI<` et sans `mcause`.
- La même composition a été vérifiée avec une destination de tableau chaîne (`A$(0)=RIGHT$(TEXT$,4)+"!"`) ; le guest produit `RABI!` sans diagnostic. Le registre et le contrat `BASIC_LANGUAGE.md` reflètent désormais aussi cette combinaison et les constructeurs explicites `CHR$`/`STR$` déjà couverts par le payload.
- L’audit de `RENUM` a révélé que la seconde opération rescannait `line_lengths` avec l’ancien index et ne trouvait aucun record déplacé logiquement. Le payload parcourt désormais les 256 records par leur numéro courant, puis réécrit dans `0x82060c00` les cibles `GOTO`/`GOSUB`/`THEN`/`ON` avant publication ; deux `RENUM` successifs et `GOTO 30` passent sous QEMU.
- La mise au point de cette réécriture a aussi révélé trois défauts d’assembleur target-side : slots vides publiés, émission décimale de `120` en `1200`, et branches hors portée après l’ajout du scanner. Ils sont corrigés ; Hammurabi, `LEN`, les tableaux chaîne et les trois scénarios `RENUM` sont verts.
- J’ai fermé la lacune de capacité de `RENUM` : une première passe réécrit chaque record dans le scratch sans publication, mesure la longueur et refuse toute sortie de plus de 111 octets. Les 256 numéros courants sont sauvegardés dans `0x82060e00`; en cas de refus, ils sont restaurés et le buffer d’entrée `x18` est réinstallé avant le retour à `READY>`. Le nouveau scénario `renum-capacity.sh` a d’abord révélé puis validé la correction du fault de reprise ; les scénarios RENUM nominal, contrôle répété et Hammurabi restent verts.
- J’ai repris `stepidp` après avoir constaté que son rapport complet existait déjà mais n’explicitait pas assez le chemin de contrôle ni la pile mémoire. Chaque pas affiche maintenant `flow=sequential` ou l’annotation d’une branche/appel/retour, la pile logique, une fenêtre cible de 16 octets autour de `sp` et, lorsqu’il y a écriture, le bloc de 16 octets autour de l’adresse modifiée. Les tests host/backend ciblés passent.
- J’ai étendu `LEN` aux expressions de concaténation chaîne simples, avec copie et évaluation entièrement target-side. Le test réduit a révélé que le concaténateur publiait seulement la longueur du dernier terme dans `x11`; la routine publie maintenant la longueur totale `x9`. Le scénario dédié couvre `LEN("R"+"V64")`, `LEN(S$)+LEN(LONGTEXT$)` et `LEN(LONGTEXT$+"!")` sous QEMU. Une interaction reste ouverte dans la séquence historique complète combinant davantage de résolutions de tableaux et d’appels `LEN`; elle n’est pas masquée ni promue en régression verte.
- J’ai poursuivi la généralisation avec `ASC` sur les concaténations simples. Le premier essai a exposé une adresse de destination mal construite (`0x82060038` au lieu de `0x82060830`) que `LEN` ne révélait pas puisqu’il ne lisait que `x11`; le buffer target-side est maintenant construit par `lui 0x82060` puis `2040+56`. `ASC("R"+"V64")`, `ASC(TEXT$+"!")` et les erreurs de chaîne vide passent sous QEMU. Les fonctions chaîne restantes ne sont pas encore généralisées.
- 2026-08-03 — J’ai remplacé les probes spécifiques de `LEN`, `ASC` et `VAL` par un résolveur d’expression chaîne commun dans le payload. La première version s’arrêtait au premier `)`, ce qui cassait `LEN(A$(1))`; un compteur de parenthèses hors guillemets, borné à huit niveaux, corrige ce cas. Les scénarios QEMU `LEN`, `ASC`, `VAL` et concaténation sont maintenant verts. Cette tranche ne prétend pas encore fournir un lexer à tokens générique : elle stabilise d’abord le contrat target-side `{adresse,longueur}` avant la migration des dispatchs de fonctions et d’`INSTR`.
- 2026-08-03 — J’ai migré `INSTR` vers le résolveur d’expression chaîne commun. Le premier opérande demande désormais un délimiteur virgule de niveau zéro (`x30=1`), le second la parenthèse externe (`x30=0`); les virgules dans les guillemets ou les parenthèses imbriquées restent dans l’opérande. QEMU valide `INSTR("HAM"+"MURABI","MUR")`, une virgule littérale et une seconde concaténation. Un essai avec `LEFT$` dans l’opérande reste volontairement documenté comme dette du parseur de termes de découpe.
- 2026-08-03 — Le premier test `INSTR(LEFT$(...),...)` a révélé non pas un problème de délimitation, mais une collision de retour : le résolveur commun réutilise le concaténateur global appelé par le concaténateur englobant. J’ai réservé `x8+2480` pour sauvegarder ce retour pendant une découpe composée. Le scénario `INSTR` valide maintenant aussi `LEFT$` dans le premier opérande, sans pile hôte ni sortie simulée.
- 2026-08-03 — Une tentative de rendre `LEFT$` réentrant dans le résolveur commun a cassé la régression `PRINT` : le cadre partagé du concaténateur ne suffit pas à empiler correctement tous les retours. Je l’ai retirée après preuve QEMU. La dette est précise : intégrer les fonctions de découpe exige une pile de contextes statique, pas une nouvelle cellule isolée.
- 2026-08-03 — Clarification d’audit : l’entrée précédente annonçant la validation de `LEFT$` dans `INSTR` décrivait un essai intermédiaire et est rétractée. Le commit ne réserve pas `x8+2480`; la preuve finale de cette tranche couvre seulement les deux opérandes composés de termes littéraux, ainsi que les virgules et parenthèses protégées.
- 2026-08-03 — J’ai introduit une pile target-side de huit cadres de concaténation de 288 octets, avec buffer de concaténation distinct par niveau, et huit cadres de métadonnées pour les résolutions. Les régressions QEMU `ASC`, `VAL`, `LEN`, `INSTR`, `PRINT` et concaténation repassent. `LEFT$` imbriqué dans `INSTR` reste volontairement non validé : son résolveur de découpe doit encore être branché sur cette pile sans perturber `PRINT`.
- 2026-08-03 — J’ai tenté de faire consommer aux découpes le résolveur d’expression commun. QEMU a immédiatement reproduit une régression `PRINT` et `INSTR(LEFT$(...),...)` restait incorrect ; le changement a été retiré après vérification. La pile de contextes reste conservée pour `LEN`, `ASC`, `VAL` et les opérandes simples d’`INSTR`, tandis que l’adaptateur des découpes doit préserver leur ABI historique avant d’être généralisé.
- 2026-08-03 — Un second audit de la composition a identifié une collision supplémentaire : un résolveur imbriqué pouvait remplacer l’adresse de destination publique du concaténateur englobant (`x8+1976`). Le cadre de 288 octets sauvegarde maintenant aussi cette destination. Les régressions `PRINT`, concaténation et `INSTR` simple sont vertes; `INSTR(LEFT$(...),...)` reste non résolu et n’est pas marqué comme supporté.
- 2026-08-03 — Après sauvegarde de la destination publique, j’ai rejoué le branchement du résolveur commun dans `LEFT$/RIGHT$/MID$`. `PRINT` est resté vert, mais `INSTR(LEFT$(...),...)` échoue encore au contrôle spécifique d’`INSTR`. Le branchement a été retiré; la correction de destination est conservée et testée séparément.
- 2026-08-03 — J’ai essayé un adaptateur plus limité : appeler l’analyseur historique de `LEFT$/RIGHT$/MID$` uniquement pour le premier opérande d’`INSTR`, puis reprendre le résolveur commun pour le second. Le test QEMU a montré une erreur dans le contrôle de fin de la découpe (`string_concat_slice_error`, caractère attendu `)` absent); le patch et son test étendu ont été retirés. Conclusion : l’ABI de reprise (`x21`) de ces routines doit être spécifié et testé séparément avant tout nouveau branchement.
- 2026-08-03 — Première migration effective vers une reconnaissance table-driven : quatre descripteurs target-side de 16 octets (`LEN`, `ASC`, `VAL`, `INSTR`) sont chargés à `0x82062000`. `parse_atom` compare désormais le nom et exige `(` avant de sélectionner l’évaluateur existant; les noms de fonctions en minuscules sont normalisés dans la ligne cible. Les tests QEMU `LEN`, `ASC`, `VAL`, `INSTR`, concaténation et `PRINT` restent verts. Les autres fonctions utilisent encore le dispatch historique.
- 2026-08-03 — Une tentative d’étendre immédiatement la table aux fonctions mathématiques a fait dépasser la portée de branches existantes dans le payload assembleur; le mini-assembleur a rejeté le programme avant exécution. J’ai retiré cette extension et vérifié que `LEN` repasse. La suite devra soit ajouter une relaxation/veneer de branches au monitor, soit migrer les fonctions par lots plus petits avec mesure de taille à chaque étape.
- 2026-08-03 — J’ai finalement déplacé le probe table-driven en fin de payload, puis étendu la table à 18 entrées : `LEN`, `ASC`, `VAL`, `INSTR`, `COS`, `ABS`, `ATN`, `EXP`, `LOG`, `TAN`, `INT`, `SIN`, `SGN`, `TRUNC`, `FRAC`, `MOD` et `SQR`. Les stubs locaux restent des branches courtes et les transferts vers les évaluateurs utilisent `jal`, ce qui évite de déplacer les branches historiques. Les tests QEMU de numérique, `ATN`, `TAN`, `LEN`, `INSTR` et `SQR` passent; `RND` reste volontairement sur son dispatch spécialisé car il accepte aussi la forme nue.
- 2026-08-03 — J’ai ajouté temporairement un scénario QEMU `INSTR(LEFT$(...),...)` et placé des points d’arrêt sur `string_concat_assign`, `string_concat_slice_term` et `string_concat_error`. La preuve montre que le buffer source de concaténation contient bien `LEFT$("HAMMURABI",4)` à l’entrée; la composition échoue ensuite lors de la reprise du curseur/du retour dans le concaténateur. Le scénario de diagnostic a été supprimé sans commit : les régressions nominales restent inchangées, et la prochaine correction devra introduire un adaptateur de reprise explicite plutôt qu’une nouvelle cellule globale.
- 2026-08-03 — J’ai réservé `+272` dans chaque cadre de concaténation pour le retour de `string_concat_slice_term` lorsque la profondeur est non nulle; les appels directs continuent d’utiliser `x8+2000`. Les scénarios QEMU de découpe directe et de concaténation repassent. Cette isolation de retour ne suffit pas encore à rendre `INSTR(LEFT$(...),...)` vert : la dette de curseur reste ouverte et n’est pas masquée.
 2026-08-03 — J’ai isolé puis corrigé la vraie régression `INSTR(LEFT$(...),...)` : après une découpe imbriquée, `x7` était réutilisé pour calculer l’adresse du cadre et revenait donc comme pointeur de cadre au lieu du buffer résultat. Le cadre sauvegarde maintenant ce pointeur à `+280`, en plus du retour à `+272`. Le cas exact est ajouté au test QEMU `string-instr-expression`; découpe littérale et concaténation repassent également.
- 2026-08-03 — Audit d'architecture MiniBASIC : le parseur numérique par niveaux de priorité compose déjà les expressions imbriquées sans énumérer leurs combinaisons. Le payload reste toutefois hybride : source ASCII parcourue par curseur, table partielle de fonctions et probes historiques pour certains mots-clés. La documentation distingue désormais explicitement cette généricité effective de la cible future lexer à tokens/parseur commun ; `INSTR(LEFT$(...),...)` reste une régression ouverte et n'est pas promue.
- 2026-08-03 — La correction définitive a confirmé l’invariant manquant : `string_concat_slice_term` doit préserver le `x31` de l’évaluateur numérique englobant, écrasé par l’évaluation du compteur de découpe. Le cadre utilise `+280` pour ce retour, le pointeur de résultat reste dans `x9` pendant le calcul du cadre, et la profondeur directe utilise `x8+2464`. Le scénario `INSTR(LEFT$(...),...)` et les régressions découpe/concaténation/PRINT passent sous QEMU.
- 2026-08-03 — J’ai fait passer les noms alphabétiques majuscules et minuscules par le reconnaisseur table-driven. Les premiers essais ont trouvé trois invariants réels : les branches B dépassaient leur portée, le repli perdait `x5` et `x29`, puis les métadonnées de table écrasaient `x28` utilisé par `store_atom`. Enfin, l’adresse d’entrée était cumulée au lieu d’être recalculée depuis `base + index*16`. Après correction, QEMU valide les fonctions numériques `ABS`, `TRUNC`, `FRAC`, `MOD`, ainsi que le repli des tableaux `B(1)+C(2)`.
- 2026-08-03 — J’ai ajouté une table target-side de 25 mots-clés de statements (`GOTO`, `FOR`, `PRINT`, `IF`, `DATA`, `DIM`, etc.). La première tentative de veneers directs a révélé la limite de labels du mini-assembleur et des contrats implicites de `x6`, `x7`, `x27`, `x28` et `x29` dans les handlers historiques. La tranche finale est donc une préreconnaissance générique : elle normalise la casse, vérifie longueur et délimiteur, sauvegarde/restaure le contexte dans `x8+2560..2600`, puis délègue au dispatch legacy. QEMU valide le payload principal, le chemin numérique, `IF`, `DATA/READ`, `INSTR` et `PRINT`; le test principal exerce aussi `print` en minuscules.
