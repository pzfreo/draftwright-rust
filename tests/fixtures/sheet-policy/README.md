CTC-03's reviewed invalid H6 plan is an input to the recovery regression, not an
expected-output oracle. It was captured by the earlier release executable from
nist/nist_ctc_03_asme1_rc.stp (native recognition), at A3 1:5. The 108.0 slot
length line intersects the 2x R9.5 callout. Its bound source hash is checked
against a fresh corpus Analysis. gzip has mtime=0 for reproducibility.

The regression requires a complete valid drawing and no exhausted fallback.
The selection-unit test separately proves that an invalid seed's lower material
crossing count cannot reject a clean candidate. No part-name branch exists in
the planner.
