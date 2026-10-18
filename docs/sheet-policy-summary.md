# Release sheet census versus current main

Before: `origin/main` `51d8f9f5`. After: release implementation including merge `71452a78`.

227 drawable inputs; the same unresolved input is refused on both sides. Counts classify the primary paper.

| Paper | Before | After |
| --- | ---: | ---: |
| A4 | 114 | 151 |
| A3 | 88 | 48 |
| A2 | 6 | 10 |
| A1 | 13 | 12 |
| A0 | 6 | 6 |
| Sheets | 227 | 227 |
| Details | 2 | 8 |

Every paper up-move (the paired Plans retain full candidate evidence):

| Part | Before | After | Reason |
| --- | --- | --- | --- |
| corpus/cadgenbench/threaded_connector_109.step | A3 1:5 / 1 / 0 | A2 1:2 / 1 / 0 | Established readable whole-part drawing retained on A2 at 1:2. smaller ordinary fit A3 at 1:5 refused [0 unreadable step spans, 0 openings below 2 mm, 0 text/arrow fit failures, 2 depiction gaps below 0.7 mm, 0 dimension-lane failures] |
| corpus/mfcadpp/10049.step | A4 1:1 / 1 / 0 | A3 2:1 / 1 / 0 | Established readable whole-part drawing retained on A3 at 2:1. smaller ordinary fit A4 at 1:1 refused [0 unreadable step spans, 1 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures] |
| corpus/mfcadpp/10098.step | A4 1:2 / 1 / 0 | A1 5:1 / 1 / 0 | A4 1:2 fill 11.8% (2 hard layout findings, 0 additional view conflicts, 0 additional material crossings, 0 annotation conflicts, 1 text/arrow fit failures, 1 dimension-lane failures, columns); A3 1:2 fill 5.4% (2 hard layout findings, 0 additional view conflicts, 0 additional material crossings, 0 annotation conflicts, 1 text/arrow fit failures, 1 dimension-lane failures, columns); A2 1:2 fill 2.6% (2 hard layout findings, 0 additional view conflicts, 0 additional material crossings, 0 annotation conflicts, 1 text/arrow fit failures, 1 dimension-lane failures, columns) |
| corpus/mfcadpp/10146.step | A3 2:1 / 1 / 0 | A2 2:1 / 1 / 0 | A4 1:1 fill 18.0% (4 hard layout findings, 0 additional view conflicts, 0 additional material crossings, 0 annotation conflicts, 0 text/arrow fit failures, 2 dimension-lane failures, columns); A3 1:1 fill 8.5% (1 hard layout finding, 0 additional view conflicts, 0 additional material crossings, 0 annotation conflicts, 0 text/arrow fit failures, 1 dimension-lane failures, columns) |
| corpus/mfcadpp_holdout/119.step | A4 1:1 / 1 / 0 | A3 2:1 / 1 / 0 | Established readable whole-part drawing retained on A3 at 2:1. Alternative search refused against the fitted drawing. |
| fixtures/parts/boss_circle.step | A4 1:2 / 1 / 0 | A3 1:1 / 1 / 0 | Established readable whole-part drawing retained on A3 at 1:1. smaller ordinary fit A4 at 1:2 refused [1 unreadable step spans, 6 openings below 2 mm, 0 text/arrow fit failures, 0 depiction gaps below 0.7 mm, 0 dimension-lane failures] |

6 parts gain details. The complete before/after inventory is in the CSV.


55 drawings change paper, scale, sheet count or detail count. Fully readable drawings: 154 → 185. Every remaining readability failure has an explicit refusal.

No lost measurement facts, lost PMI/GDT claims or new drawing-finding identities. Direct dimensions and independently verified common-axis geometry retain the measurement inventory. See [verification and PNG index](sheet-policy-verification.md).

Every part whose paper, main scale or sheet count changes is listed in [the change table](sheet-policy-changes.md). No part gains sheets. GRM-01 is an additional external review input, outside the fixed 227-part census.
