//! Writes `fixtures/sample.fig` to disk — a real, minimal, binary-Kiwi
//! `.fig` file built by `figma_import::fixtures::sample_fig_bytes()`
//! (see that module for what it does and does not represent).
//!
//! Run: `cargo run --example make_fixture`

fn main() {
    let bytes = figma_import::fixtures::sample_fig_bytes();
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    std::fs::create_dir_all(&out_dir).expect("create fixtures dir");
    let out_path = out_dir.join("sample.fig");
    std::fs::write(&out_path, &bytes).expect("write fixture");
    println!("wrote {} bytes to {}", bytes.len(), out_path.display());
}
