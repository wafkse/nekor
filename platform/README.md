# Platform Support

This directory contains platform data, templates, and generated build inputs.

## Platform roots

A platform is registered by name and root directory in `nekor.toml`. Platform inheritance is also declared there through the optional `base` field. Platform data does not declare or discover parent files.

Each platform root may contain a `data` directory. Every `**/*.kdl` file below that directory contributes to the same platform layer. Files in one layer are merged strictly, so duplicate terminal definitions are errors rather than precedence rules.

A derived platform overlays its resolved base platform. Inherited node annotations, scalar representation annotations, and structural forms remain constraints on derived values.

## Typed KDL data

Platform data uses KDL 2. Reserved numeric annotations such as `(u64)` preserve exact representation through parsing and template projection. Application node annotations such as `(virtual-address)` retain semantic identity for validation and tooling.

Lists use dashed child nodes so list identity does not depend on element count.

### Naming style

Platform data generally uses `snake_case` for structural tree nodes and `kebab-case` for leaf configuration keys and semantic annotations. This is an authoring convention rather than a parser rule. The semantic document always preserves the exact spelling written in KDL.

MiniJinja templates use ordinary attribute traversal for structural nodes and bracket indexing for kebab-case leaves. For example, `kernel.schedule.queue["section-name"]` accesses the `section-name` leaf without introducing any name translation layer.

## Template expansion

MiniJinja templates use the `**/*.jinja` suffix. `cargo nekor platform --name <platform> template build` renders templates from the selected platform root against the fully resolved typed platform document.

The build system supplies `workspace()` and `platform()` functions to templates. Generated files are written beside their template source with the `.jinja` suffix removed.
