//! Fuzz the parser path directly: raw bytes in, `import` out.
//!
//! This is the code path a hostile upload takes before any HTTP layer
//! runs — a panic, infinite loop, or unbounded allocation here is a
//! remote DoS.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = figma_import::fig::import(data, "fuzz.fig");
});
