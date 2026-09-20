//! Builds a real, minimal, binary-Kiwi `.fig` byte stream (DOCUMENT →
//! CANVAS → FRAME → [TEXT, RECTANGLE]) — not a mock or a placeholder JSON
//! blob. This reproduces the exact wire-encoding technique `op-figma`'s
//! own upstream test suite
//! (`crates/op-figma/src/binary_e2e_tests.rs` in ZSeven-W/openpencil) uses
//! to build a fixture from scratch, because op-figma ships no bundled
//! sample `.fig` file — the upstream repo doesn't include one either, it
//! builds this shape programmatically in its own e2e test. The technique
//! (Kiwi varint/string/field wire writer, `fig-kiwi` magic + deflate
//! chunk framing) is a straightforward implementation of Figma's public
//! `.fig` container format, not copied source. Used by both
//! `examples/make_fixture.rs` (writes `fixtures/sample.fig` to disk) and
//! `tests/import_test.rs` (drives it through the real router in-process)
//! — see README.md "Testing" for what this fixture does and does not
//! exercise.

use std::io::Write;

#[derive(Default)]
struct W {
    out: Vec<u8>,
}

impl W {
    fn byte(&mut self, b: u8) {
        self.out.push(b);
    }
    fn var_uint(&mut self, mut v: u32) {
        loop {
            let mut byte = (v & 127) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 128;
            }
            self.out.push(byte);
            if v == 0 {
                break;
            }
        }
    }
    fn var_int(&mut self, v: i32) {
        self.var_uint(((v << 1) ^ (v >> 31)) as u32);
    }
    fn var_float(&mut self, v: f32) {
        let stored = v.to_bits().rotate_left(9);
        if stored & 0xff == 0 {
            self.out.push(0);
        } else {
            self.out.extend_from_slice(&stored.to_le_bytes());
        }
    }
    fn string(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
        self.out.push(0);
    }
    fn field(&mut self, name: &str, type_code: i32, is_array: bool, value: u32) {
        self.string(name);
        self.var_int(type_code);
        self.byte(if is_array { 1 } else { 0 });
        self.var_uint(value);
    }
}

fn build_schema() -> Vec<u8> {
    let mut w = W::default();
    // Definition indices (referenced by later `field()` type codes):
    // 0 GUID, 1 Vec, 2 Matrix, 3 ParentIndex, 4 TextData, 5 NodeType,
    // 6 NodeChange, 7 Message.
    w.var_uint(8);

    w.string("GUID");
    w.byte(1);
    w.var_uint(2);
    w.field("sessionID", -4, false, 0);
    w.field("localID", -4, false, 0);

    w.string("Vec");
    w.byte(1);
    w.var_uint(2);
    w.field("x", -5, false, 0);
    w.field("y", -5, false, 0);

    w.string("Matrix");
    w.byte(1);
    w.var_uint(6);
    for m in ["m00", "m01", "m02", "m10", "m11", "m12"] {
        w.field(m, -5, false, 0);
    }

    w.string("ParentIndex");
    w.byte(1);
    w.var_uint(2);
    w.field("guid", 0, false, 0);
    w.field("position", -6, false, 0);

    // struct TextData { characters: string } — the real Figma NodeChange
    // schema also carries `characterStyleIDs` / `styleOverrideTable` for
    // per-run styling, but op-figma's text_mapper.rs
    // (`build_content`, see src/text_mapper.rs) falls back to
    // `TextContent::Plain(characters)` whenever those two are absent, so
    // a minimal single-field struct is a faithful (not fudged) encoding
    // of an unstyled Figma text run — verified against the actual
    // decode path in src/kiwi.rs (`decode_struct`: positional fields, no
    // presence bitmap) before wiring this in, not assumed.
    w.string("TextData");
    w.byte(1);
    w.var_uint(1);
    w.field("characters", -6, false, 0);

    // enum NodeType — TEXT=5 added alongside the DOCUMENT/CANVAS/RECTANGLE
    // set the upstream e2e fixture (binary_e2e_tests.rs) uses.
    w.string("NodeType");
    w.byte(0);
    w.var_uint(5);
    w.field("DOCUMENT", 0, false, 1);
    w.field("CANVAS", 0, false, 2);
    w.field("FRAME", 0, false, 3);
    w.field("RECTANGLE", 0, false, 4);
    w.field("TEXT", 0, false, 5);

    w.string("NodeChange");
    w.byte(2);
    w.var_uint(7);
    w.field("guid", 0, false, 1);
    w.field("parentIndex", 3, false, 2);
    w.field("type", 5, false, 3);
    w.field("name", -6, false, 4);
    w.field("size", 1, false, 5);
    w.field("transform", 2, false, 6);
    w.field("textData", 4, false, 7);

    w.string("Message");
    w.byte(2);
    w.var_uint(1);
    w.field("nodeChanges", 6, true, 1);

    w.out
}

struct NodeSpec {
    local_id: u32,
    parent: Option<(u32, u32, &'static str)>,
    type_value: u32,
    name: &'static str,
    size: Option<(f32, f32)>,
    transform: Option<[f32; 6]>,
    text: Option<&'static str>,
}

fn encode_guid(w: &mut W, session: u32, local: u32) {
    w.var_uint(session);
    w.var_uint(local);
}

fn encode_node(w: &mut W, n: &NodeSpec) {
    w.var_uint(1);
    encode_guid(w, 0, n.local_id);
    if let Some((s, l, pos)) = n.parent {
        w.var_uint(2);
        encode_guid(w, s, l);
        w.string(pos);
    }
    w.var_uint(3);
    w.var_uint(n.type_value);
    w.var_uint(4);
    w.string(n.name);
    if let Some((x, y)) = n.size {
        w.var_uint(5);
        w.var_float(x);
        w.var_float(y);
    }
    if let Some(m) = n.transform {
        w.var_uint(6);
        for v in m {
            w.var_float(v);
        }
    }
    if let Some(t) = n.text {
        w.var_uint(7);
        w.string(t);
    }
    w.var_uint(0);
}

fn build_data() -> Vec<u8> {
    let nodes = [
        NodeSpec {
            local_id: 1,
            parent: None,
            type_value: 1, // DOCUMENT
            name: "Doc",
            size: None,
            transform: None,
            text: None,
        },
        NodeSpec {
            local_id: 2,
            parent: Some((0, 1, "a")),
            type_value: 2, // CANVAS
            name: "Landing Page",
            size: None,
            transform: None,
            text: None,
        },
        NodeSpec {
            local_id: 3,
            parent: Some((0, 2, "a")),
            type_value: 3, // FRAME (hero section container)
            name: "Hero",
            size: Some((1440.0, 640.0)),
            transform: Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            text: None,
        },
        NodeSpec {
            local_id: 4,
            parent: Some((0, 3, "a")),
            type_value: 5, // TEXT
            name: "Headline",
            size: Some((800.0, 64.0)),
            transform: Some([1.0, 0.0, 80.0, 0.0, 1.0, 96.0]),
            text: Some("Ship faster with Tasmanian Cloud"),
        },
        NodeSpec {
            local_id: 5,
            parent: Some((0, 3, "a")),
            type_value: 4, // RECTANGLE (a CTA button plate)
            name: "CTA Button",
            size: Some((160.0, 48.0)),
            transform: Some([1.0, 0.0, 80.0, 0.0, 1.0, 200.0]),
            text: None,
        },
    ];
    let mut w = W::default();
    w.var_uint(1);
    w.var_uint(nodes.len() as u32);
    for n in &nodes {
        encode_node(&mut w, n);
    }
    w.var_uint(0);
    w.out
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn build_fig(schema: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"fig-kiwi");
    buf.extend_from_slice(&[0, 0, 0, 0]);
    for chunk in [schema, data] {
        let comp = deflate(chunk);
        buf.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        buf.extend_from_slice(&comp);
    }
    buf
}

/// Build the fixture bytes. Shared by `examples/make_fixture.rs` and
/// `tests/import_test.rs` so both exercise the identical byte stream.
pub fn sample_fig_bytes() -> Vec<u8> {
    build_fig(&build_schema(), &build_data())
}
