# Detail schema cost after main integration

The regenerated 26 standalone MCP schemas occupy **46,328 compact UTF-8 bytes** per
session. Both the generator and the Rust tools/list test use the exact strict bound
`< 46,329`. Main's regenerated schemas occupy 42,124 bytes, inside its 42,500 budget.
No schema validation was removed to meet a byte target.

The detail record adds **4,116 bytes**: **343 bytes in each of twelve Spec schemas**
(eleven tool inputs and get_spec output). The same compaction with the record removed
measures 42,212 bytes. Eight supported detail placement-view names add another **88
bytes** in item_options output. Total detail vocabulary cost relative to regenerated
main is therefore **4,204 bytes**. The previous branch measured 46,868 bytes; main's
compaction saves 540 bytes while also retaining authored dimension and line-work forms.

| Record component | Per schema | All twelve | Why it is needed |
| --- | ---: | ---: | --- |
| `around` and its required constraint | 98 | 1,176 | Stable feature/face ID or nonempty string array; rejects missing targets, empty arrays and nonstring members. |
| Optional `from` | 96 | 1,152 | Nullable principal/section source, constrained by a strict end-anchored pattern; rejects iso, malformed sections and trailing newlines. |
| Optional `scale_factor` | 54 | 648 | Numeric enlargement strictly greater than one. |
| `detail` property and closed nullable record structure | 95 | 1,140 | Distinguishes an absent/null request from an object and refuses unsupported geometry fields such as bounds. |
| **Total record** | **343** | **4,116** | |

These are marginal removal measurements with the same generator and optimizers;
removing all three fields leaves the 95-byte structural cost per schema. Retain main's
`spec_read`, `hoist_objects`, `group_properties`, `inline_single_use`, `share_repeated`
and short local definitions. The disjoint string/array type avoids union branches;
the section-name pattern avoids an enumerated list. The strict 42,500 limit leaves only 375 usable
bytes beyond main, while the typed record alone costs 4,116. Replacing typed input
records with permissive sparse objects would fit better but would discard validation
that the detail schema regressions require, so the exact measured bound is retained.

Reproduce the total with `python3 tools/generate_mcp_schemas.py`; the served-byte Rust
check is `tools_explain_their_operations_without_source_and_fit_schema_budget` in
`tests/engine_version.rs`. Generated JSON is never hand-merged.
