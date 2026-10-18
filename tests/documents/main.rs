// Each original module keeps its helper instances, including private cache statics.
#![allow(clippy::duplicate_mod)]

#[path = "../label_text.rs"]
mod label_text;
#[path = "../linework_spec.rs"]
mod linework_spec;
#[path = "../notes.rs"]
mod notes;
#[path = "../output.rs"]
mod output;
#[path = "../pmi_drawings.rs"]
mod pmi_drawings;
#[path = "../png.rs"]
mod png;
#[path = "../render_caches.rs"]
mod render_caches;
#[path = "../render_panics.rs"]
mod render_panics;
#[path = "../sections.rs"]
mod sections;
#[path = "../sheets.rs"]
mod sheets;
#[path = "../sheets_lint.rs"]
mod sheets_lint;
#[path = "../sheets_render.rs"]
mod sheets_render;
#[path = "../sheets_xref.rs"]
mod sheets_xref;
#[path = "../spec_gdt.rs"]
mod spec_gdt;
#[path = "../spec_notes.rs"]
mod spec_notes;
#[path = "../surface_texture.rs"]
mod surface_texture;
#[path = "../tables.rs"]
mod tables;
#[path = "../text.rs"]
mod text;
#[path = "../timezone.rs"]
mod timezone;
#[path = "../title_block.rs"]
mod title_block;
#[path = "../title_block_material.rs"]
mod title_block_material;
