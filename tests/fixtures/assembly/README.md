# Assembly fixtures

`assembly.step` and `assembly_pin_pmi.step.gz` are described in `tests/assembly_parts.rs`.

`vslot-20x20-100.step.gz` (#212) is pzfreo/dw-examples' `steps/freecad/vslot-20x20-100.step`,
gzip-compressed without changing its STEP bytes: the FreeCAD library's
[Mechanical Parts/Profiles EN/OpenBuilds/V-Slot-20x20-100mm.step](https://github.com/FreeCAD/FreeCAD-library/blob/f5ca01cdcaa2f69b9295a6382719c2d90628780c/Mechanical%20Parts/Profiles%20EN/OpenBuilds/V-Slot-20x20-100mm.step),
added by Charles Fox (`a305030944`, 2026-09-29). It holds three coincident copies of one
profile (part 0 placed twice, and part 1, `Pad`), 55 faces each. Uncompressed SHA-256:
`9c98f6a68872f4ed5f1cc2b514cf3e70ded179b4c1c90a4761adcef3903e874f`.

The V-slot profile is an OpenBuilds design, shared under
[CC BY-SA 4.0](https://openbuildspartstore.com/open-source-license); the FreeCAD library
licenses it under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). This file is
shared under CC BY-SA 4.0 with that attribution.
