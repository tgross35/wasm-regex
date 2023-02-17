# wasm-regex

A very rudimentary demo of Rust's regex capabilities used in WASM. Follows
[Mozilla's guide](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm).

## Running

First, if you don't already have it, install [wasm-pack](https://github.com/rustwasm/wasm-pack)
with `cargo install wasm-pack`.

Next, inside the main folder, run `wasm-pack build` (add `--release` for full optimization).

To build an even smaller wasm file (for releases), use:
`wasm-pack build --release --no-typescript --features none -Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort`.

Install the necessary packages with `npm install`.

Finally, run `npm run serve` to get the site up and going locally.
