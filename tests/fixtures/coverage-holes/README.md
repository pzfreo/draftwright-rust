Fixtures copied unchanged from Python draftwright's versioned evaluation corpora
(`tests/fixtures/evaluation`, revision 586628b96db6779454d499c7d92d37f1aa8f1df9).
The synthetic evaluation fixtures are CC0-1.0; see that repository's
`corpus-double-d-bores-v1.json`, `corpus-countersinks-v1.json`, and
`corpus-turned-steps-v1.json` for their authored expectations.

They exercise real recognition independently of the annotation mutation tests:
rotated flats, different bore axes, blind profiles, ordinary round bores,
countersinks of different sizes, topology order, deburrs and an external cone.

The `profile-*.step`, `three-shafts.step`, `asymmetric-shafts.step`, and
`countersink-two-face.step` fixtures are CC0 synthetic examples made for this
port. The profiles are 10 mm extrusions of respectively a trapezoid, triangle
and irregular pentagon. Shafts have adjoining ø30 × 20 and ø20 × 20 bands,
with their axes at x = (-80, 0, 80) or (-32, 0, 90). The two-face plate is
50 × 50 × 12 with an ø6 bore and ø14, 90° seats at both mouths.
