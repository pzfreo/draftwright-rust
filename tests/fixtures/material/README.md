GRM-03 material regression (#101)

`GRM-03_thumbwheel_drive_screw_AP242_PMI.step.gz` is the PMI tester's original
GRM-03 AP242 input, gzip-compressed without changing its STEP bytes. It was supplied
in the task's scratch `derived-inputs/` directory, rather than the geometry-only
`gramel/GRM-03_thumbwheel_drive_screw.step` in the corpus.

Uncompressed SHA-256:
`4b6462b9cc9f0d419250933bd77fb305f9cfebb7ec2b3f377008732876010a21`.

The material is CAx-IF Material Identification §4.1, the same property form as the
working `hw_string_post_tapped_intent` (Brass CW614N) and
`hw_thumbwheel_thread_knurl_intent` (Stainless steel 303) PMI fixtures:

```step
#1994 = REPRESENTATION('material name',(#1995),#769);
#1995 = DESCRIPTIVE_REPRESENTATION_ITEM('CZ121 / CW614N / CuZn39Pb3',
  'Free-machining leaded brass');
#1996 = PROPERTY_DEFINITION('material property','material name',#5);
#1997 = PROPERTY_DEFINITION_REPRESENTATION(#1996,#1994);
```

The pinned haecceity reader returns `Material { id: "CZ121 / CW614N / CuZn39Pb3",
name: Some("Free-machining leaded brass"), density: None }`, owned by part 0,
with provenance `[1994, 1995, 1996, 1997]`. Draftwright already projects that to
`Material { source_id: "material:#1996", designation: "CZ121 / CW614N / CuZn39Pb3",
... }` on origin/main at `1e5a87a1` (which includes #108). Its sole owned material
fills the title block. It does not require a new material reader or kernel change.

The old source census wording "read by haecceity; not projected to a draftwright
record" describes the absence of a PMI annotation record, not the absence of this
separate material fact. #108 suppresses the false undrawn warning when the title
block carries that fact. `grm03_ap242_material_reaches_the_title_block_with_its_source`
checks the kernel reading, draftwright fact, planned cell, rendered text, source
annotation and coverage lint against this actual file. Keep the fixture here so
the 22-file PMI comparison/survey inventory remains the same.
