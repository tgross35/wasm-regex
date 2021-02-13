use regex::Regex;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn test(text: &str, reg_exp: &str) -> bool {
    let re = Regex::new(reg_exp).unwrap();
    re.is_match(text)
}
