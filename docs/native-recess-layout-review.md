# Native recess layout repair

The comparison is against `699acae7` (released recognition), using the unchanged
30-part manifest and quality baseline. The initial native results are retained in
`/tmp/section-recess-review/after-final`; the original drawings are in `before`.
New information does not excuse moving an existing item into clutter. Counts below
are the initial regression cells, not approved baseline changes.

| Part | Initial worsened cells (before → native) | New information | Existing-item regression / cause |
| --- | --- | --- | --- |
| 1000 | ink 0→1; extension contacts 0→1 | Rectangular pocket size/depth and two locations; hex pocket; circular seat and axis locations | New hex callout escapes the side band; new location/leader contact. Existing envelope carries the coincident 48.4 span with both claims. |
| 10000 | ink 5→6; through-feature 0→1 | Pocket size/depth and x/y locations | Existing x shoulder exceeds its band after lane insertion; existing hole y witness crosses the newly recognised pocket. |
| 10007 | ink 0→1 | Two seat arcs, common diameter/run and axis locations | Sheet changes A3 2:1→A4 1:1; existing x shoulder exceeds its band. |
| 10020 | ink 1→4; through-feature 0→3; extension contacts 5→7 | Two side pockets with size/depth and in-plane locations; hex pocket | New locations exceed side band; new pocket leader crosses its peer location and pocket. Existing hole/rung witnesses cross newly recognised recesses. |
| 1004 | through-feature 2→4 | Edge-open side pocket size/depth | Existing hole x/y witnesses cross newly recognised recess footprints; no new leader is responsible for these two findings. |
| 10046 | ink 0→1 | Side-opening pocket size/depth and two locations | Sheet changes A3 2:1→A4 1:1; new front callout exceeds its band. |
| 10047 | ink 0→4; through-feature 0→1; extension contacts 0→3; off-span 1→3 | Two round-bottom blind slots and their locations, pocket and hex pocket | Three new callouts exceed bands; pocket leader crosses overall width; new slot location crosses an existing slot; linear label settlement loses span association. |
| 10049 | ink 0→1; through-feature 0→4; extension contacts 0→2 | Three pockets, two with explicit locations | New side callout exceeds band and crosses another pocket; new location crosses the large pocket. Existing height rungs also cross that newly recognised footprint. |
| 10060 | ink 6→7; extension contacts 5→6 | Side pocket size/depth and two locations | New side location exceeds band; existing hole leader length grows 19.547→25.606 mm. |
| 10063 | ink 1→3; through-feature 0→2; extension contacts 3→4 | Front and plan pockets with locations; hex pocket | Existing envelope width and new front callout exceed band. Existing hole leader and witness cross the newly recognised plan pocket. |
| NIST CTC02 | through-feature 6→8; extension contacts 2→4 | Two seat arcs and their axis locations | New axis witnesses cross holes. Existing blend leader grows 56.134→64.139 mm; extra extension contacts are not justified by the new information. |
| NIST FTC06 | repeats 0→2; ink 5→9; through-feature 3→5; extension contacts 3→4; off-span 0→2 | Four channel widths: two 3.1 side, two 12.7 plan | Equal widths create two repeated-label groups. Existing hole pitch leaves band and chamfer text gains an extension contact; new widths leave bands, cross holes and lose text association. |
| NIST FTC08 | repeats 1→2; ink 1→6; through-feature 11→13; extension contacts 3→12 | Two seat arcs and axis locations; two 3.1 channel widths | Sheet changes A2 1:2→A0 1:1. Existing slot-radius leader grows 10→55.251 mm, gains hidden crossings and text contacts; new axis witnesses cross existing holes/slots. Equal channel widths repeat. |
| NIST FTC09 | through-feature 2→7 | Four seat arcs and shared axis locations | Existing height witnesses cross a newly recognised pocket. New side axis location crosses the hole bank; new front locations cross holes/slots. New seat leader runs beside ink 13.428 mm. |

Every non-NIST row requires correction, including intersections with newly
recognised footprints. Dense NIST increases may only be accepted for extra
information after hard findings and existing-item regressions are removed.
No baseline cell has been re-recorded.
