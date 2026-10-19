# Round 5 sheet census versus current main

Before: `origin/main` `51d8f9f5`. After: `7c5a3ec8`.

227 drawable inputs; the same unresolved input is refused on both sides. Counts classify the primary paper.

| Paper | Before | After |
| --- | ---: | ---: |
| A4 | 114 | 164 |
| A3 | 88 | 37 |
| A2 | 6 | 9 |
| A1 | 13 | 11 |
| A0 | 6 | 6 |
| Sheets | 227 | 227 |
| Details | 2 | 9 |

Every paper up-move (the paired Plans retain full candidate evidence):

| Part | Before | After | Reason |
| --- | --- | --- | --- |
| corpus/mfcadpp/10049.step | A4 1:1 / 1 / 0 | A3 2:1 / 1 / 0 | A3 2:1 makes the 1.5 mm opening readable; the A4 1:1 trial remains below the 2 mm printed-opening floor. |
| corpus/mfcadpp/10146.step | A3 2:1 / 1 / 0 | A2 2:1 / 1 / 0 | A2 2:1 restores the 19 mm pocket-location witness clearance; the A3 trial leaves only 3.6 mm. |
| corpus/mfcadpp_holdout/119.step | A4 1:1 / 1 / 0 | A3 2:1 / 1 / 0 | A3 2:1 clears A4’s short step, two text/arrow failures, witness-lane failure and near-frame ink. |
| fixtures/parts/boss_circle.step | A4 1:2 / 1 / 0 | A3 1:1 / 1 / 0 | A3 1:1 makes the six small holes and short step readable; A4 1:2 fails those measured floors. |

7 parts gain details. The complete before/after inventory is in the CSV.


67 drawings change paper, scale, sheet count or detail count. Fully readable drawings: 154 → 183. Every remaining readability failure has an explicit refusal.

No lost measurement facts, lost PMI/GDT claims or new drawing-finding identities. Direct dimensions and independently verified common-axis geometry retain the measurement inventory. See [verification and PNG index](sheet-policy-verification.md).

Every paper, main-scale or sheet-count change is in [the change table](sheet-policy-changes.md). No part gains sheets. The independent border audit reports zero contacts across all 227 drawings.
