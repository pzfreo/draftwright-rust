# Detail schema cost after recognition and label-text integration

The 26 regenerated standalone MCP schemas occupy **46,656 compact UTF-8 bytes**
per session. The generator and `tests/engine_version.rs` enforce the smallest
strict integer bound, **< 46,657**. Generated JSON comes exclusively from
`tools/generate_mcp_schemas.py`.

Main at `54704307` (#80 and #81) regenerates to **42,465 bytes**. Removing only
our typed detail record from the merged generator, then rerunning the same
compaction, yields **42,553 bytes**. The detail record's marginal share is therefore
**4,103 bytes**, with another **88 bytes** for eight placement-view names.
Total detail vocabulary cost relative to main is **4,191 bytes**.

| Spec schemas | Record bytes each | Subtotal |
| --- | ---: | ---: |
| get_item, get_spec, inspect_part, item_options, list_problems, render, render_pdf inputs | 343 | 2,401 |
| apply_ops, preview_ops, list_items inputs | 340 | 1,020 |
| view_sheet input and get_spec output | 341 | 682 |
| **All twelve** | | **4,103** |

The previous accepted branch measured 46,328 bytes, including a 4,116-byte
record. Main's iterative compaction and root schema targets lower the record's
marginal cost by 13 bytes while retaining recognition declarations and label text.
The final merged total is 328 bytes larger than the previous accepted branch.

The record remains closed and nullable, requires `around` as a string or nonempty
string array, constrains `from` to principal/section names with a strict end anchor,
and requires numeric `scale_factor > 1`. No validation was removed for size.
Main's sparse input maps, typed read forms, recognition declarations, label fields,
iterative compaction and root reference targets are retained.

Measurements are removal experiments with identical optimizers, so sharing can
make the marginal cost differ across standalone schemas. Reproduce the final total
with `python3 tools/generate_mcp_schemas.py`; the Rust served-byte check is
`tools_explain_their_operations_without_source_and_fit_schema_budget`.
