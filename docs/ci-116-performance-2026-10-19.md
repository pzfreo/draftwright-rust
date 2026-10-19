# PR #116 CI repair — 19 October 2026

The release checks and measurements below cover the CI repair on `cx/drawn-values`.
The comparison checkout is `origin/main` at `51d8f9f58c82` (a detached worktree of
this repository), built with its own `CARGO_TARGET_DIR`.

## Changes

Automatic final nominal repairs run on the selected drawing, rather than every
rejected paper/scale candidate. Authored inventory edits retain their candidate
fit judgment, and the public unrecovered planner still finishes its returned
sheet. Sessions use that same recovered entry point and keep counting actual
recovered sheet runs; pin replay therefore uses the same finished drawing as
the CLI. Permission to add sheets preserves a complete single-sheet result.
New final collision reports receive ordinary placement fixes; reports
whose dimension owners were consolidated away are withdrawn. A selected automatic
no-table ladder may try one opposite location band: the ordinary planner reserves
its space from the start, using recovery’s validity/completeness ordering.

Compact comparisons have an isolated planning scope: their printed wants cannot
reuse the live precise wants. Established recess corridors are reused where the
selected drawing retains those statements; other comparisons are measured on
the selected paper. Candidate generation that establishes silhouette steering
still precedes scoring of the incumbent.

Native recognition prepares exact visibility for preferred projections of
fractional freeform callouts and small established bore inventories. Recess drawings retain their established corridor
recipe and conservative projected edges; turned stock retains its silhouette
proof. Visibility indexes are lazy. A compact label on an established ink run is
retained before launching a futile precise reroute. Numeric formatting selects
the decimal count before formatting, preserving the previous noise tolerance and
three-place cap (1,500,000 differential comparisons agreed).

Lint formatting no longer reads planner comparison state. The recovery replay
fixture uses the current three-field policy key; its cross-view veto assertion
is unchanged. Label-lane examples now check their modelled values: 6.925, 8.854,
10.854; 4.458, 1.233; 14.031; and string_post's 1 mm terminal band.

## Timing method

Release CLI `plan` calls (including native analysis and recognition) use the same
four STEP inputs on both revisions. Darwin
`proc_pid_rusage` supplies retired instructions; this remains meaningful when
the fan governor pauses a process with SIGSTOP. Runs alternate revisions. The
final table records minima over three runs; wall time is also retained in the
local JSON evidence. Build time is excluded. No timeouts or budgets are raised.

The original branch binary (`694225b1`) measured 391.102, 535.150, 551.247 and
22.546 billion instructions on FTC-08, CTC-02, cgb245 and string_post respectively;
main measured 100.696, 233.905, 26.644 and 4.828 billion in that comparison.

## Drawing decisions

The faithful display still prints 4.75 rather than 4.8, while stripping floating
noise (247.65000001 prints 247.65). A single GRM-02 flat prints 7.4 without A/F;
proved opposed flats retain A/F. Neither change alters modelled measurements.

Two unchanged model-ink runs retain compact labels in the final quality set:

| Part and dimension | Printed | Modelled | Reason |
| --- | --- | --- | --- |
| FTC-08 `d:f:fillet:7/fillet.radius` | `8× R1.5` | Eight radii of 1.524 mm, fillets 5–12 | The established shaft has a pre-existing ink run; retain its compact text and shaft instead of launching an unsuccessful precise reroute. |
| 1004 `d:f:hole:1/bore.diameter` | `⌀7.4 ↧ 5.9` | Diameter 7.44654 mm; depth 5.86 mm | The established shaft has a pre-existing ink run. Its compact statement preserves that corridor; the faithful statement would be `⌀7.447 ↧ 5.86`. |

This CI repair changes no leader-route baseline, frozen reference, threshold,
timeout, or budget.
CHANGELOG.md is unchanged from origin/main.

## Validation

Formatting and strict workspace/all-target release clippy pass. The unfiltered
nextest inventory matches all eight integration binaries, 141 reachable Rust
files and 1,305 integration tests. The complete suite is not run: execution is
limited to the library and the requested core, dimensions, rendering and audits
binaries: all 1,514 tests pass (268 library; core 279, dimensions 336, rendering
473, audits 158). The final run takes 467.423 seconds at eight nextest threads,
with acceptance running alongside part of it. All three originally reported
library failures pass; lint's guard and the recovery veto assertion are unchanged.

The final 30-part quality run passes all 450 status-table cells. Its unchanged
leader-route baseline passes 145 callouts, including CTC-03 and FTC-10: no extra
hidden/section/cutting-plane crossings or new runs over 2 mm; three unchanged
pre-existing runs remain. Label lanes pass with faithful labels checked against
modelled values. Pin cell 3-1 passes all 30 parts; FTC-10 has zero ink collisions
both before and after pinning. The final pinned FTC-10 PNG was inspected.

Local evidence is under `target/ci116/shared-acceptance/`,
`target/ci116/shared-nextest.log`, `target/ci116/shared-clippy.log` and
`target/ci116/final-nextest-list.json`. The final timing table follows below.

## Final timing minima

Each cell is its independent minimum over three alternating runs. Wall completion
is polled at 5 ms to avoid coarse rounding on string_post. CPU time excludes
SIGSTOP pauses; retired instructions provide a second load-independent comparison.
No test, build or acceptance job was running during these measurements.

| Part | Main wall s | Branch wall s | Wall change | Main CPU s | Branch CPU s | CPU change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| nist_ftc_08 | 4.219 | 4.283 | +1.5% | 4.297 | 4.355 | +1.3% |
| nist_ctc_02 | 8.940 | 9.940 | +11.2% | 9.949 | 10.949 | +10.0% |
| cgb245 | 1.174 | 1.310 | +11.6% | 1.249 | 1.383 | +10.7% |
| string_post | 0.227 | 0.240 | +5.7% | 0.287 | 0.294 | +2.4% |
| Total | 14.559 | 15.773 | +8.3% | 15.781 | 16.980 | +7.6% |

| Part | Main billion instructions | Branch billion instructions | Change |
| --- | ---: | ---: | ---: |
| nist_ftc_08 | 100.564 | 105.313 | +4.7% |
| nist_ctc_02 | 233.842 | 259.717 | +11.1% |
| cgb245 | 26.605 | 29.888 | +12.3% |
| string_post | 4.821 | 5.034 | +4.4% |
| Total | 365.832 | 399.953 | +9.3% |

The four-part total is within about 10% of main on all three measures. The
largest individual wall increase is cgb245 at 11.6% (10.7% CPU); CTC-02 is
11.2% wall (10.0% CPU). These small residual costs remain visible in the table.
The original branch’s 1,500.045 billion instructions fall to 399.953 billion, a
73.3% reduction on the same four inputs. Raw samples and plans are retained in
`target/ci116/final-timings/`; the executable comparison is `b36d9627` against
`origin/main` (`51d8f9f58c82`) in its separate target directory.
