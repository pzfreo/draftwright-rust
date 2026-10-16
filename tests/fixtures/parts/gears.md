`gear_blank.step` is a radius 8 mm, width 10 mm centred cylinder, generated with
build123d `export_step(Cylinder(8, 10), ...)`, matching the blank in Python's
`tests/test_issue_1086_declared_gears.py`. Its tip diameter intentionally does not
follow from the authored example (Python accepts it).

`gear_wheel.step.gz` is a losslessly compressed copy of Python draftwright's
`tests/fixtures/issue_1058_wheel_rh.step` (SHA-256
4911c06426f0ceeedc198416e058aabc1c1a6a65e9e766eca7efed5484a27cda).
It provides actual full-wire evidence for the 13-tooth wheel and its double-D bore.
No normative gear values are read from that evidence in production.
