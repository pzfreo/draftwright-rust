# Stage H lessons for Stage I

Written 2026-10-12 from the Stage H reviews (workflow wf_27f3ab14-79a). Each Stage I brief that
retries a Stage H stream starts from its branch and this note, and carries its checks as
acceptance items. The process rules at the end go into every Stage I brief.

## h-chains (`cxg/h-chains` at 0d46bc3, not merged)

**Tried:** remove closed dimension chains and duplicates; a new `closed_dimension_chain` lint;
maintainer decision (iii), keeping leaders and text pads off extension lines.

**Keep:** the chain work. GRM-03, CTC-05, FTC-07, 10047's (5.1) reference and string_post are
better by eye, and 11 of its 12 `h5_chains` tests fail on the old main. The leader rule does
improve ctc_01, cgb241, 10033 and GRM-03.

**Why it failed:** the general leader rule moves a callout to the first route with no pad
contact that is "no worse" on hard, crossings, feature crossings, outside and text. None of
those counts leader length, crossings of hidden or cutting-plane lines, or a leader running
along a model or extension line. So it trades a 1 mm pad touch for a long leader across the view.
- nist_ftc_06: `2x ⌀14.3 THRU …` went from above the view with a short leader to below it, its
  leader crossing the side view, the A-A cutting-plane line and hidden circles.
- 10020: the ⌀1.8 boss callout now crosses the view interior, its leader collinear with an
  extension line.
- 10060: the ⌀5.3 THRU leader is vertical at x=97.70, collinear with the riser outline at
  x=97.64 for 14.5 mm. It can't be told from the object line. Introduced in fix1 (5b540e6).
- Also open from round 2: 10047's side view still has the closed 18.6 / 4.5 / 23.1 chain
  (`dims/redundancy.rs` interval() and `lint/redundancy.rs` span() skip step_level ladders); the
  lint misses CTC-05-style duplicates (no external_span evidence); `lint/plates.rs:440` accepts
  any riser at the plate's coordinate (restrict to risers whose z-range overlaps the plate).

**Checks to add first (each must fail at 0d46bc3):**
1. A moved callout's leader is not longer than before by more than a small margin, and crosses
   no more hidden, section or cutting-plane lines.
2. No leader runs within 0.3 mm of a model, hidden or extension line for more than 2 mm (the
   reviewer's script over the 30 sheets; ftc_06, ctc_04 `4x ⌀10 THRU`, 10060, 10020).
3. 10047's lint reports the 18.6 / 4.5 / 23.1 closure, and the plan no longer has it.

## h-extlines (`cxg/h-extlines` at 6ca293e, not merged)

**Tried:** put dimensions on the open side so extension lines don't cross features; route
leaders around concave outlines.

**Keep:** extension crossings fell 171 → 124 (27.5%) with no part worse; FTC-08 ink 46 → 25,
leader crossings 28 → 14, extension crossings 39 → 12.

**Why it failed:** the target was one third (≤ 114), and the branch commits
`tools/check_extension_routes.py`, which exits 1 on its own HEAD. The concave-leader item was
never shown: `profiles::silhouettes` refuses tori (most fillets), so every NIST, CGB, GRM,
sm-hanger and string_post view is `leader_outline_unverified` and never gets the penalty. The
real blocker for the rest is sheet size (CTC-02 and FTC-09 would need A0/A1), which multi-sheet
changes.

**Under the dense-NIST bar:** 27.5% with no part worse is a real gain, and the maintainer
accepted dropping the one-third threshold. **But it was backed out at the second-pass merge
(2026-10-13):** on top of main it adds about 39% to planning instructions (the same
`footprints::Shape::crossed` / `leaders::measure_at` hotspot I0 targets), and six tests in other
streams' features failed: G3's radius-centre table no longer appears on any corpus part,
H7's interior retry never runs on the plate fixture, `known_leaders.json` needs new verdicts, and
CTC-01 moves to A4 1:10 where the HEX leader crosses f:hole:1 and a slot width's text leaves its
span. The design issue: H6 picks the sheet from tries planned at a named sheet and scale, so the
open-side bands rarely reach the final drawing, and pins follow the try's frames. The WIP is on
local branch `stageh-pass2-extlines-wip` (b0bb77c). Retry after I0 (speed) and I1 (sheets).

**Checks and follow-ups:**
1. The committed check passes on the branch's own HEAD (always).
2. Silhouettes cover tori, so concave parts get outline evidence; then show a real concave part
   with fewer leader crossings.
3. Baseline ladders stay nested when dimensions move side (FTC-08, FTC-10 x ladder): no
   extension line crosses a rung's dimension line that a nested order would not.
4. `model::recover`'s second `clear_extensions` call gets the same guard as `plan_with`, or goes.
5. `prismatic_axes` / `silhouettes` in the Analysis get a schema version bump.

## g7b-recesses (`cxg/g7b-recesses` at 1e1488c, review pending)

**What happened:** the takeover fixed three blockers by **removing the recess schedule**, not by
correcting it (the schedule headed its columns X/Y/Z MIN but printed seat axis midpoints). The
quality gate still has 4 of 300 cells worse: 10020, 10047, nist_ctc_02.

**Lesson:** removing output is not a fix unless the information still reaches the sheet some
other way. Integration checks that every recess location is still dimensioned; if not, it counts
as lost information under the dense-NIST bar.

**Open decisions (for the maintainer, from the takeover):** 10020 needs a larger sheet, scale
or a detail view; 10047's 93 mm blind-slot labels need two-line callouts, shorter wording or a
larger sheet. Multi-sheet (Stage I) may settle both.

## h-span (merged candidate, passed)

Most of text_off_span's fall (151 → 16) comes from its threshold counting the default 5.7 mm
gap. The strict figure is 43 → 16. Quote both.

## Process rules for every Stage I brief

1. **Metrics can be gamed.** A metric that improves while the sheet gets worse is a failure.
   Every acceptance metric comes with a look at the named parts' sheets, and the reviewer
   compares before and after renders.
2. **The "no worse" gate must count what a draftsman sees:** leader length, crossings of hidden,
   section and cutting-plane lines, and collinearity with model or extension lines.
3. **Removing output is not a fix.** Dropping a table, note or dimension needs a stated reason,
   and the reviewer checks the information still reaches the sheet.
4. **Red before green against the base,** not against the stream's own WIP commit.
5. **A committed acceptance script must pass on the branch's own HEAD.** A target that is not
   met goes in not_done with the measured figure, not in a failing check.
6. **Rebase onto the current main before review** (Stage H branches missed the CLA and DXF
   commits).
7. **Time on a quiet machine.** The 150 ms session gate fails under load; latency claims are
   measured with load below about 8, or with `DRAFTWRIGHT_LATENCY_GATE=0` and a separate timed
   run.
8. **Dense-NIST bar:** new hard findings, lost information and failing tests block; cosmetic
   leader and crowding issues on dense NIST parts don't.
9. **Unattended:** never wait for answers; record judgement calls in `docs/next-session.md`.

## Codex drops: resume, don't lose the round

In Stage H, 7 Codex runs ended early: 6 on "Selected model is at capacity" (g3c impl, g6c fix1,
h-chains impl, h-interior impl and fix1, h-sheetfit fix1) and 1 on a transport error (h-tables
fix1). Each cost a whole round, and several streams went to Claude takeover partly because of
that. Codex can usually continue straight away.

The Stage I driver therefore does this: when `codex exec` exits, read the last events of its
`--json` log. If they are `turn.failed` with "at capacity", a 5xx, or "stream disconnected", run
`codex exec resume <thread_id> "Continue the task from where you stopped."` with the same `-C`,
`-c` and `--add-dir` options (thread_id from the first `thread.started` event), appending to a
new log. Wait 30 s, then 2 min, then 5 min between attempts, up to 6 resumes. Only after that
does the round count as failed. The 4-hour limit counts Codex's working time, not the waits.
