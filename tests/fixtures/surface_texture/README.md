# Surface texture fixtures

AP242 surface textures (ISO 10303-1110 `surface_texture_representation`, ISO 1302) written by
hand into copies of `tests/fixtures/parts/manufacturing_pmi.step.gz` (a plate carrying written
AP242 PMI, whose part-wide texture is `Ra 3.2`, 'any process allowed'), in the entity shape
haecceity's PMI writer makes for the editor (`draftwright-editor/crates/intent/src/pmi_writer.rs`,
`SurfaceTexture`). `tests/surface_texture.rs` reads them.

- `plate_part_ra1_6.step.gz`: the part-wide texture's `characteristic value` changed from 3.2 to
  1.6 µm (#1267).
- `plate_face_ra0_8.step.gz`: the texture's value changed to 0.8 µm and its two property
  definitions (#1262 'surface texture', #1268 'surface_condition') moved from the part (#4) to a
  new shape aspect of the plate's bottom face: `#1274=SHAPE_ASPECT('','',#4,.T.)` with
  `#1275=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#1274,#10,#559)`.

They live outside `parts/` and `pmi/step/`, so the single-sheet byte guard's case set is unchanged.
