# MCP schema context budget

## Step 1: baseline on cx/mcp-spec-schema

The 26 standalone schemas on main total **42,681 compact UTF-8 JSON bytes**.
Spec and spec_read contribute **19,353 bytes (45.34%)**; everything else is
**23,328 bytes (54.66%)**. Input Spec shapes cost **17,309 bytes** and the
get_spec output spec_read shape costs **2,044 bytes**. Eleven input schemas and get_spec's output carry Spec
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

## Steps 2 and 3: outline and on-demand schema

The new total is **34,389 bytes**, including the fourteenth tool, `spec_schema`:
a saving of **8,292 bytes (19.43%)** against main's 42,681. The budget covers only
inputSchema/outputSchema values, just as before. The full Spec response is paid
only when requested; tool descriptions and transport framing remain outside this
particular measurement. No detail-view changes from #73 are present in this branch.

| Tool | Input | Output | Total |
| --- | ---: | ---: | ---: |
| analyse | 173 | 943 | 1116 |
| apply_ops | 1578 | 3091 | 4669 |
| get_item | 1119 | 1417 | 2536 |
| get_spec | 1072 | 1299 | 2371 |
| inspect_part | 1397 | 2528 | 3925 |
| item_options | 1082 | 1245 | 2327 |
| list_items | 1918 | 1561 | 3479 |
| list_problems | 1406 | 1414 | 2820 |
| parts | 106 | 587 | 693 |
| preview_ops | 1542 | 2989 | 4531 |
| render | 1431 | 523 | 1954 |
| render_pdf | 1117 | 523 | 1640 |
| spec_schema | 62 | 420 | 482 |
| view_sheet | 1323 | 523 | 1846 |
| TOTAL | 15326 | 19063 | 34389 |

The outline explicitly names all current top-level Spec keys. Object fields are
`sheet`, `views`, `dimensions`, `annotations`, `features`, `faces`,
`dimension_schemes`, and `precision`; `binding` and `sheets` are nullable objects.
`notes`, `tables`, `gears`, and `feature_declarations` are arrays.
`iso_scale_factor` is a nullable number and `envelope_tolerance` a nullable string.
Short enums describe `schema`, `dimension_source`, and `hole_table`.
All fields remain optional and top-level `additionalProperties` remains `false`.
The description directs the client to `spec_schema {}` and `item_options`.

`spec_schema` needs no part registration or cache access. Its `spec` result is the
standalone draft-2020-12 schema (`schemas/spec.json`); `spec_read` is the fragment
`#/$defs/spec_read` within that same document, naming get_spec's Spec value shape.
The full schema describes every current model field, including fields main's
advertised sparse schema omitted. It describes structure; the engine still checks
references, geometry, engineering constraints and edits.

`schemas/mcp-validation.json` retains all eleven original input preflight schemas
unchanged as JSON values. They are generated separately from the outline and
never appear in tools/list. This preserves existing validation and exact guidance;
serde/model/geometry checks are unchanged. The engine fingerprint now includes all
schema JSON files, including the full and private documents.

The generator and Rust budget test use the measured ceiling. The Python regression
adds a nested sheet field to both full and private schemas and verifies that every
advertised schema remains byte-identical. A separate assertion checks that the
outline's keys match the Rust Spec fields. Rust tests exercise valid literal Specs
and fixture examples in authored/read shapes, known structural defects and exact
preflight guidance. The independent Python checker uses the actual tool response
and draft-2020-12 validation for all checked-in Spec documents, including seven
recorded structurally invalid examples that must be refused.

The browser/editor methods and TypeScript declarations do not change. The WASM
documentation now points schema consumers to the full document instead of assuming
MCP outlines describe nested fields. There is no drawing_api_reference file in this
checkout; the older compatibility MCP executable already uses its own shallow
Spec description and does not consume schemas/mcp.
