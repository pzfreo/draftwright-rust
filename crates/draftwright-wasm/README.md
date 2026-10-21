# draftwright-wasm

Browser bindings for draftwright's live drawing session. JSON documents cross the JS boundary
as strings; SVG and PDF exports are `Uint8Array`. Uses the bundled drawing fonts.

See [the API and consumption guide](https://github.com/pzfreo/draftwright-rust/blob/main/docs/wasm.md).
Release assets contain this package; there is no npm publication.

Use `placementOptions()` to discover free placement: each compatible item's `offset` is
`"callout"` or `"linear"`. `calloutOffset(address)` returns JSON
`{view, tip:[x,y], bend:[x,y], offset:[dx,dy], members:[{centre:[x,y], radius, span?:{start,sweep}}]}`; `labelOffset(address)` returns
`{view, anchor:[x,y], label:[x,y], offset:[dx,dy]}` for linear dimensions. Coordinates are
paper mm, y up. Pin a dropped bend with `{offset: bend - tip, pinned: true}`, or a linear
label origin with `{offset: label - anchor, pinned: true}`. Returned offsets pin the current
position without movement. These methods throw the same API errors as the live Rust session.

Callout members are the projected attachment sites in paper mm (y up). An arc
includes `span` with counterclockwise `start` and `sweep` in radians. A dragged
callout attaches radially to the member nearest its elbow, retaining the automatic
member on ties. The automatic `tip` remains the offset origin; changing members
keeps the elbow, shoulder and label position. Reapplying the automatic offset
retains the exact automatic ink.
