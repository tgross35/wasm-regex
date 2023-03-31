//! js-related helpers
#![allow(unused)]

use wasm_bindgen::prelude::*;

/// Quick macro to print to the console for debugging
#[cfg(feature = "js-console")]
macro_rules! console {
    ($($tt:tt)*) => {
        crate::log(&format!($($tt)*))
    };
}

#[cfg(not(feature = "js-console"))]
macro_rules! console {
    ($($tt:tt)*) => {};
}

#[cfg(not(test))]
#[wasm_bindgen]
extern "C" {
    /// Log to the js console
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// For testing, override the wasm log and just use stderr
#[cfg(test)]
fn log(s: &str) {
    eprintln!("{s}");
}

/// Use the console as the panic handler. Must be called from js to
#[wasm_bindgen]
#[cfg(feature = "js-console")]
pub fn debug_init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

/// nop without our feature
#[wasm_bindgen]
#[cfg(not(feature = "js-console"))]
pub fn debug_init() {}
