# MCP schema context budget

## Step 1: baseline on cx/mcp-spec-schema

The 26 standalone schemas on main total **42,681 compact UTF-8 JSON bytes**.
Spec and spec_read contribute **19,353 bytes (45.34%)**; everything else is
**23,328 bytes (54.66%)**. Eleven input schemas and get_spec's output carry Spec
(twelve schema documents across eleven tools).

Reproduce with `python3 tools/measure_mcp_schemas.py`. This uses the same compact
serialization as the Rust session budget. Attribution replaces each
`properties.spec` value schema with `{}`, then removes local reference targets
that become unreachable. Targets still referenced outside Spec remain charged
to Other. Thus the Spec column includes exclusively owned reference targets and
their keys, but excludes the `spec` property name and does not double-count shared
shapes. Tool descriptions, names and transport envelopes are outside this budget.
This is a marginal byte attribution, not a sum of pretty-printed file lengths.

| Tool | Input | Output | Spec + spec_read | Other | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| analyse | 173 | 943 | 0 | 1116 | 1116 |
| apply_ops | 2269 | 3091 | 1578 | 3782 | 5360 |
| get_item | 1815 | 1417 | 1587 | 1645 | 3232 |
| get_spec | 1768 | 2471 | 3631 | 608 | 4239 |
| inspect_part | 2081 | 2528 | 1556 | 3053 | 4609 |
| item_options | 1778 | 1245 | 1587 | 1436 | 3023 |
| list_items | 2591 | 1561 | 1519 | 2633 | 4152 |
| list_problems | 2090 | 1414 | 1556 | 1948 | 3504 |
| parts | 106 | 587 | 0 | 693 | 693 |
| preview_ops | 2236 | 2989 | 1578 | 3647 | 5225 |
| render | 2127 | 523 | 1587 | 1063 | 2650 |
| render_pdf | 1813 | 523 | 1587 | 749 | 2336 |
| view_sheet | 2019 | 523 | 1587 | 955 | 2542 |
| TOTAL | 22866 | 19815 | 19353 | 23328 | 42681 |
