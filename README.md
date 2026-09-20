# figma-import

One-way `.fig` → normalized JSON conversion service. **Not** a Figma
embed, a live design-editor integration, or an MCP/daemon client — it
is a small HTTP wrapper around
[`op-figma`](https://github.com/ZSeven-W/openpencil/tree/main/crates/op-figma)'s
real binary `.fig` (Kiwi format) parser, for a one-time import step
that feeds another system's page/block generation pipeline (built for
[Tasmanian-Cloud/website-builder](https://github.com/Tasmanian-Cloud/website-builder)).

## Why `op-figma` and not a hand-rolled parser

`op-figma` is the Rust port of openpencil's Figma importer: it
unzips/splits the `.fig` container, decodes the embedded Kiwi binary
schema, builds the node tree, and converts it into openpencil's own
canonical `jian_ops_schema::PenDocument` shape (frames/groups/
rectangles/text/images/etc, with position, size, and — where present —
fill/stroke/effect data). It is MIT licensed and, critically for this
use case, **standalone**: its own `[dependencies]` are
`jian-ops-schema` (a plain schema/types crate — serde, schemars,
`ts-rs`, `thiserror`; no daemon or editor code), `flate2`, `ruzstd`,
and `base64`. The openpencil editor/daemon crates
(`op-editor-core`, `op-editor-ui`, `op-host-services`,
`op-pen-loader`) appear only in `[dev-dependencies]`, gated to
`op-figma`'s own `#[cfg(test)]` module and its `examples/`
— they are never pulled in by a normal consumer. This was verified by
reading `crates/op-figma/Cargo.toml` directly (not assumed from
docs), and confirmed by this repo's own `cargo build`, which compiles
`op-figma` + `jian-ops-schema` and nothing editor-shaped.

`Cargo.toml` pins `op-figma` to a specific upstream commit (`rev =`,
not a floating branch), so this service's parsing behaviour doesn't
silently drift when openpencil changes.

## API

### `GET /health`

`{"ok": true}`.

### `POST /import`

Multipart form upload. Fields:

- `file` (required) — the `.fig` file bytes.
- `name` (optional) — overrides the document name; otherwise the
  upload's filename is used, falling back to `"Figma Import"`.

Success (`200`):

```jsonc
{
  "ok": true,
  "fileName": "sample.fig",
  // op-figma's own converted PenDocument, unmodified — a real
  // {version, name, pages: [{id, name, children: [...]}], ...} tree.
  // This service does not invent a parallel schema; callers walk
  // pages[].children the same way any PenDocument consumer would.
  "document": { "version": "1", "pages": [ /* ... */ ] },
  // Non-fatal warnings op-figma itself produced during conversion.
  "warnings": [],
  "summary": {
    "pageCount": 1,
    "totalNodeCount": 3,
    "nodeCountsByType": { "frame": 1, "text": 1, "rectangle": 1 },
    "textContent": ["Ship faster with Tasmanian Cloud"],
    "imageRefs": [{ "nodeId": "fig_9", "name": "Photo", "src": "data:image/png;base64,..." }],
    "unsupportedFeatures": [ "..." ]
  }
}
```

Errors (`422` / `413`) return `{"ok": false, "code": "...", "message": "..."}`.
Codes: `missing_file`, `too_large`, `unknown_format`,
`clipboard_json_not_supported`, `parse_failed`, `multipart_error`.

Only the **binary** `.fig` format is accepted (Figma's real export —
either the bare Kiwi container or the more common ZIP-wrapped
`canvas.fig`). Figma's clipboard-JSON export (`.fig.json`, produced by
copy-pasting layers rather than exporting a file) is explicitly
**not** handled by this endpoint — `op-figma` supports it separately
via a different, shallower code path (`figma_clipboard_to_nodes`) that
returns a flat node list, not a full document; wiring that in is a
deliberate non-goal here since the target workflow is "upload a `.fig`
file", not "paste layers".

## Fidelity — what actually carries over

This is the honest boundary, not a marketing list. `summary.unsupportedFeatures`
is always returned (not conditional on what a given file used) so a caller
can show it before the user even uploads:

**Carries over well:**
- Page / frame / group hierarchy and names
- Text content (plain runs; styled/mixed-run text collapses to its
  concatenated plain string — see `src/fig.rs::extract_text_content`)
- Absolute position (`x`, `y`) and size (`width`, `height`) per node
- Node kind (rectangle / ellipse / line / polygon / path / text / image / frame / group)
- Image references (as `data:` URLs op-figma resolved, or the original
  reference string when it couldn't decode the blob)

**Does not carry over** (either `op-figma` doesn't map it into
`PenDocument`, or the mapping exists but isn't exact):
- Blur/shadow effect *parameters* beyond presence/type
- Gradient angle/transform precision (stop color + position survive; exact Figma matrix does not)
- Figma variables / design tokens (no variable-binding step in this pipeline)
- Auto-layout's exact gap/padding/alignment semantics (approximated via flex, or raw geometry — see `layoutMode`)
- Component/instance property overrides beyond the base component tree
- Boolean operations / vector network detail beyond an outline-path fallback
- Prototyping and interaction wiring (flows, transitions, overlays)

None of these are silently dropped-and-hidden: they're absent from the
JSON because `op-figma` doesn't produce them, and this service never
fabricates a placeholder value in their place.

## Testing

`tests/import_test.rs` drives the real Axum router in-process
(`tower::ServiceExt::oneshot`) against a genuine binary `.fig` byte
stream built by `src/fixtures.rs`. `op-figma` ships no bundled sample
`.fig` — neither does upstream openpencil; its own end-to-end test
(`crates/op-figma/src/binary_e2e_tests.rs`) builds one from scratch the
same way. `src/fixtures.rs` reproduces that exact Kiwi wire-encoding
technique (magic + deflate-chunked schema/data, varint/string field
writer) — verified against `op-figma`'s actual `src/kiwi.rs` decoder
(struct vs. message field framing) and `src/text_mapper.rs` (the
`textData.characters` shape) before being wired in, not assumed — to
build a small DOCUMENT → CANVAS → FRAME(Hero) → [TEXT, RECTANGLE)
fixture and assert on the real parsed output: one page, one text node
whose content round-trips exactly, one rectangle, correct per-type
counts.

```sh
cargo test                      # unit + integration tests
cargo run --example make_fixture   # writes fixtures/sample.fig to disk
cargo run --bin figma-import       # starts the service on :3000 (or $PORT)
```

Manually verified against a real running instance:

```sh
curl -F "file=@fixtures/sample.fig" http://localhost:3000/import
```

**Not tested here:** a real Figma-authored export file (this
environment had no such file and no live Figma account to produce
one). The synthetic fixture exercises the same decode paths
(container/zip detection, Kiwi schema+data decode, tree build, text +
geometry mapping) a real file would, but does not exercise every
Figma feature a real design uses — auto-layout, components, styles,
effects, gradients, boolean ops, and multi-page/multi-canvas files are
untested here. Test against a real export before depending on this in
production.

## Running

```sh
PORT=3000 cargo run --bin figma-import
# or
docker build -t figma-import .
docker run -p 3000:3000 figma-import
```

No environment variables are required. `PORT` defaults to `3000`.
Upload size cap is 512 MiB (`figma_import::MAX_UPLOAD_BYTES`) — real
`.fig` exports routinely carry hundreds of MB of embedded bitmaps.

## License

MIT, matching `op-figma`'s own license.
