# Modelled screw threads

Recognition v4 `modelled_thread` features use ISO 6410-1 conventions by default.
Only the occurrence's claimed faces are replaced, for drawing purposes, with a
smooth carrier at its crest diameter. The kernel projects that carrier, resolves
visibility, and cuts/hatches it. The source Part, face identities, dimensions and
recognition records stay unchanged. Indivisible cylindrical carriers retain their
extent outside the measured thread span. Unclaimed geometry stays in the drawing. Circular boundaries shared with an
unclaimed end face, within one pitch of the measured span, meet the smooth
carrier at its crest: a full thick root/chamfer circle would contradict the
end view's open thin root arc.

External crests (major diameter) and internal crests (minor diameter) are thick.
Roots are thin continuous lines in visible profiles and sections; internal side
views use the existing hidden-line layer. End views have a thin three-quarter
root arc open in the upper right quadrant. Section hatching reaches the crest.
Root spacing is the measured radial difference, enlarged on paper when necessary
to at least twice the 0.5 mm outline width (ISO 6410-1 3.2.1). Callouts retain the
measured sizes. Oblique termination circles project as arcs rather than diameters across openings.
Conventional curves obey view visibility, kept-half clipping,
detail crops and hidden-line settings. Both physical trimmed limits are shown.

The callout states only the designation, for example `M75×1.5`. The ISO 6410
lines depict the thread extent; no `LONG` suffix, trimmed-span text or separate
length dimension is generated. Recognition's physical trimmed span includes
partial end turns and runouts and remains measurement evidence in the plan.
It does **not** prove blind-hole depth or minimum full engagement. Source PMI
can state those manufacturing requirements independently. This follows the
extent-depicted convention in [thread-callouts.md](thread-callouts.md).

## Designation inference and author intent

A unique ISO 261 diameter/pitch pair is required (ISO 262 is its selected subset).
The pitch tolerance is 0.01 mm. External major diameters must be within 0.02 mm
of nominal. Internal roots can be truncated: a nominal diameter may be up to
`clamp(0.26 × measured pitch, 0.02, 0.4)` mm above the measured major diameter,
or at most 0.02 mm below it. These are **inference policy tolerances**, not an
ISO fit class or certified metrological uncertainty. No tolerance class is inferred.
Ambiguous or unmatched pairs print `ø<major> × P<pitch>` (for example `ø110 × P6.35`, on one line) and info
`thread_designation_measured`. Measured values use at most three decimal places
with trailing zeroes trimmed; their original values and any rounding finding
remain in the plan. Fine pitch stays explicit, as does coarse pitch.
ISO 965-1 multiple-start metric threads state lead and pitch, e.g. `M20×Ph2P1-LH`; unmatched
threads state the measured pitch and start count. Right hand is implicit; inferred metric left-hand designations end in `-LH`.

Per-occurrence overrides express intent, using the stable feature ID:

```json
{
  "features": {
    "f:modelled_thread:1": {
      "modelled_thread": {
        "depiction": false,
        "designation": "SPECIAL THREAD"
      }
    }
  }
}
```

`depiction: false` restores modelled edges and reports
`modelled_thread_not_depicted` with the reason. `designation` replaces only the
entire inferred callout text verbatim, with no generated length suffix. Omitting a
field restores the automatic decision. The existing sheet/view `thread_depiction`
controls also apply; view controls take precedence over sheet controls.
Dimensions retain the usual omission, placement, prefix/suffix and text-template
controls. Schemas and MCP item options expose the same intent fields.

In PMI `first` and `strict` modes a semantic thread attached to the occurrence
owns the callout: no second inferred callout is added. `ignore` uses recognition
alone. Existing tapped-hole and Spec-authored thread callouts keep their ordinary
PMI policy and source values.

Sources: [ISO 6410-1:1993](https://cdn.standards.iteh.ai/samples/12750/07f99becf07a454ab9fece84a0dce861/ISO-6410-1-1993.pdf),
[ISO 261:1998](https://cdn.standards.iteh.ai/samples/4165/ec50482690954e82931c52fdce3bc8bb/ISO-261-1998.pdf).

Multiple-start and left-hand syntax follows ISO 965-1:2013, clauses 12.3–12.4
([standard text](https://www.finesz.com/pic/ISO965-1.pdf)).
