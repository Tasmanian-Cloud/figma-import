//! Wraps `op-figma`'s real parser. This module does not invent a parallel
//! node schema: the response embeds `op-figma`'s own converted
//! `jian_ops_schema::PenDocument` output verbatim (via `serde_json`,
//! without depending on the `jian-ops-schema` crate directly — see the
//! note on `summarize` below for why). The `summary` block is a
//! convenience index over that same JSON, computed by walking the wire
//! representation rather than the Rust types.

use op_figma::{detect_kind, parse_fig_binary_with_images, FigFileKind, FigLayoutMode};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::error::AppError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResponse {
    pub ok: bool,
    pub file_name: String,
    /// `op-figma`'s converted document, unmodified — a real
    /// `jian_ops_schema::PenDocument` (`{version, name, pages: [...], ...}`)
    /// serialized as-is. Downstream (website-builder) maps `pages[].children`
    /// onto its own block schema; this service does not do that mapping.
    pub document: Value,
    /// Non-fatal conversion warnings `op-figma` itself produced (e.g. a
    /// Figma feature it fell back on).
    pub warnings: Vec<String>,
    pub summary: Summary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub page_count: usize,
    pub total_node_count: usize,
    /// Node count by `PenNode` wire `"type"` tag (`frame`, `text`,
    /// `rectangle`, `image`, ...).
    pub node_counts_by_type: BTreeMap<String, usize>,
    /// Every non-empty text run found (`TextNode.content`, `Plain` or the
    /// concatenation of a `Styled` run's segments), in document order —
    /// this is the fidelity Figma import realistically buys: content and
    /// rough structure, not pixel-perfect rendering.
    pub text_content: Vec<String>,
    /// One entry per `ImageNode`: `{ nodeId, name, src }`. `src` is
    /// whatever `op-figma` resolved it to — a `data:` URL for an embedded
    /// blob it could decode, or the original reference string when it
    /// couldn't. Callers needing a real asset host must re-upload `src`
    /// data: URLs themselves; this service does not persist images.
    pub image_refs: Vec<ImageRef>,
    /// Figma features `op-figma` does not carry into `PenDocument` at all,
    /// or that the target website-builder schema has no slot for even if
    /// it did. Always present, not conditional on what this particular
    /// file used — callers should treat these as permanently out of scope
    /// for this import path, not per-file warnings.
    pub unsupported_features: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRef {
    pub node_id: String,
    pub name: Option<String>,
    pub src: String,
}

/// Figma capabilities `op-figma` + this service do not round-trip. Kept as
/// a static list (not inferred per-file) so a caller can render an
/// accurate "what you'll lose" notice before the user even uploads.
const UNSUPPORTED_FEATURES: &[&str] = &[
    "blur/shadow effect parameters beyond presence (PenEffect carries a type, not Figma's exact radius/spread/color curve)",
    "gradients beyond linear/radial stop position + color (Figma angle/transform-matrix precision is not preserved)",
    "Figma variables/design tokens (this pipeline has no variable-binding step)",
    "auto-layout resolved to Figma's exact gap/padding/alignment semantics (layout_mode=OpenPencil approximates via flex; layout_mode=Preserve keeps raw geometry instead)",
    "component/instance overrides beyond the base component tree (instance-specific property overrides are not diffed)",
    "boolean operations and vector network detail beyond an outline path fallback",
    "prototyping/interaction wiring (Figma flows, transitions, overlays)",
];

pub fn import(bytes: &[u8], file_name: &str) -> Result<ImportResponse, AppError> {
    match detect_kind(bytes) {
        FigFileKind::Binary => {}
        FigFileKind::ClipboardJson => return Err(AppError::ClipboardJsonNotSupported),
        FigFileKind::Unknown => return Err(AppError::UnknownFormat),
    }

    let parsed = parse_fig_binary_with_images(bytes, file_name, FigLayoutMode::OpenPencil, None)
        .map_err(|e| AppError::ParseFailed(e.to_string()))?;

    let document = serde_json::to_value(&parsed.document)
        .map_err(|e| AppError::ParseFailed(format!("document did not serialize: {e}")))?;

    let summary = summarize(&document);

    Ok(ImportResponse {
        ok: true,
        file_name: file_name.to_string(),
        document,
        warnings: parsed.warnings,
        summary,
    })
}

/// Walks the serialized `PenDocument` JSON (not the Rust `PenNode` enum)
/// to build the summary. Deliberate: `PenNode`'s home crate
/// (`jian-ops-schema`) is reachable from this binary only *through*
/// `op-figma`'s git dependency on a pinned upstream commit. Adding
/// `jian-ops-schema` as a second, independent dependency of this crate
/// would resolve to a distinct Cargo unit (different source: our direct
/// git dependency vs. the path dependency baked inside op-figma's own git
/// checkout) even at an identical version, so a `PenNode` value handed
/// back by `op-figma` would not typecheck against a `PenNode` named by
/// our own `use` — a classic diamond-dependency non-unification trap.
/// Walking the JSON `Value` sidesteps it entirely and is exactly the
/// contract the wire format already guarantees (`#[serde(tag = "type")]`).
fn summarize(document: &Value) -> Summary {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut texts: Vec<String> = Vec::new();
    let mut images: Vec<ImageRef> = Vec::new();
    let mut total = 0usize;

    let page_count = document
        .get("pages")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    if let Some(pages) = document.get("pages").and_then(Value::as_array) {
        for page in pages {
            if let Some(children) = page.get("children").and_then(Value::as_array) {
                for child in children {
                    walk_node(child, &mut counts, &mut texts, &mut images, &mut total);
                }
            }
        }
    }
    // A Figma import always lands in `pages`, but walk top-level
    // `children` too in case a caller feeds a clipboard-shaped document
    // through this same summarizer in future.
    if let Some(children) = document.get("children").and_then(Value::as_array) {
        for child in children {
            walk_node(child, &mut counts, &mut texts, &mut images, &mut total);
        }
    }

    Summary {
        page_count,
        total_node_count: total,
        node_counts_by_type: counts,
        text_content: texts,
        image_refs: images,
        unsupported_features: UNSUPPORTED_FEATURES.to_vec(),
    }
}

fn walk_node(
    node: &Value,
    counts: &mut BTreeMap<String, usize>,
    texts: &mut Vec<String>,
    images: &mut Vec<ImageRef>,
    total: &mut usize,
) {
    let Some(obj) = node.as_object() else {
        return;
    };
    *total += 1;
    let ty = obj.get("type").and_then(Value::as_str).unwrap_or("unknown");
    *counts.entry(ty.to_string()).or_insert(0) += 1;

    if ty == "text" {
        if let Some(text) = extract_text_content(obj) {
            if !text.trim().is_empty() {
                texts.push(text);
            }
        }
    }

    if ty == "image" {
        let node_id = obj
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let name = obj.get("name").and_then(Value::as_str).map(str::to_string);
        let src = obj
            .get("src")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        images.push(ImageRef { node_id, name, src });
    }

    // Every container-shaped node (frame/group/…) carries its subtree in
    // `children`; walk it regardless of `type` rather than allowlisting
    // container types, so a schema addition upstream doesn't silently
    // stop being counted here.
    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        for child in children {
            walk_node(child, counts, texts, images, total);
        }
    }
}

/// `TextNode.content` is `TextContent::{Plain(String), Styled(Vec<StyledTextSegment>)}`
/// serialized `#[serde(untagged)]` — on the wire it's either a bare JSON
/// string or an array of segment objects. `StyledTextSegment` carries the
/// run text in its own `text` field (mirroring the rest of the schema's
/// naming); concatenate segments in order to recover the plain string a
/// block's `content`/`text` field wants.
fn extract_text_content(obj: &Map<String, Value>) -> Option<String> {
    match obj.get("content")? {
        Value::String(s) => Some(s.clone()),
        Value::Array(segments) => {
            let joined: String = segments
                .iter()
                .filter_map(|seg| seg.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            Some(joined)
        }
        _ => None,
    }
}
