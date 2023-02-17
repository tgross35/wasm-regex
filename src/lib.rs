//! Simple regex utility available via WASM

mod error;
use error::Error;
use regex::bytes::{Regex, RegexBuilder};
use serde::Serialize;
use std::str;
use wasm_bindgen::prelude::*;

/// Representation of all matches in some text
#[derive(Debug, Serialize)]
struct MatchSer<'a> {
    /// List of all matches
    matches: Vec<Vec<Option<CapSer<'a>>>>,
}

/// Representation of a single capture group
#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct CapSer<'a> {
    /// Optional name of the capture group
    group_name: Option<&'a str>,
    /// Index of the match within all matches
    #[serde(rename = "match")]
    match_num: usize,
    /// Index of the group within this single match
    group_num: usize,
    /// Always true for Rust
    is_participating: bool,
    /// Content of the capture group
    content: &'a str,
    /// Start index in the original string
    start: usize,
    /// End index in the original string
    end: usize,
}

/// Process specified flags to create a regex query. Acceptable flags characters
/// are `gimsUux`. Also validates the regex string
///
/// The returned bool indicates if global
fn re_build(reg_exp: &str, flags: &str) -> Result<(Regex, bool), Error> {
    // Validate the syntax is correct, error if not
    let _ = regex_syntax::Parser::new().parse(reg_exp)?;

    let mut builder = RegexBuilder::new(reg_exp);
    let mut builder_ref = &mut builder;
    let mut global = false;

    // Unicode is enabled by default, so we need to explicitly disable it
    builder_ref.unicode(false);

    for flag in flags.chars() {
        match flag {
            'g' => global = true,
            'i' => builder_ref = builder_ref.case_insensitive(true),
            'm' => builder_ref = builder_ref.multi_line(true),
            's' => builder_ref = builder_ref.dot_matches_new_line(true),
            'U' => builder_ref = builder_ref.swap_greed(true),
            'u' => builder_ref = builder_ref.unicode(true),
            'x' => builder_ref = builder_ref.ignore_whitespace(true),
            // We can panic here because the UI should only ever give us valid
            // flags
            _ => panic!("unrecognized flag"),
        }
    }

    match builder.build() {
        Ok(re) => Ok((re, global)),
        Err(e) => Err(e.into()),
    }
}

/// Run a regular expression on a block of text, returning a JSON string
///
/// # Arguments
///
/// - `flags`: apply global flags, options `imsUux`
/// - `text`: haystack to search in
/// - `reg_exp`: regular expression to match against
///
/// Returns a string JSON representation of `CapSer`
fn re_find_impl(text: &str, reg_exp: &str, flags: &str) -> Result<JsValue, Error> {
    let (re, global) = re_build(reg_exp, flags)?;
    let limit = if global { usize::MAX } else { 1 };
    let mut matches: Vec<Vec<Option<CapSer>>> = Vec::with_capacity(16);

    // If we aren't global, limit to the first match

    // Each item in this loop is a query match. Limit to `limit`.
    for (match_idx, cap_match) in re.captures_iter(text.as_bytes()).take(limit).enumerate() {
        // For each capture name, get the correct capture and turn it into a
        // serializable representation (CapSer). Collect it into a vector.
        let mut match_: Vec<Option<CapSer>> = Vec::with_capacity(re.captures_len());

        for (i, opt_cap_name) in re.capture_names().enumerate() {
            let to_push = cap_match.get(i).map(|m| CapSer {
                group_name: opt_cap_name,
                group_num: i,
                match_num: match_idx,
                is_participating: true,
                content: &text[m.start()..m.end()],
                start: m.start(),
                end: m.end(),
            });

            match_.push(to_push);
        }

        matches.push(match_);
    }

    let res = MatchSer { matches };
    Ok(serde_wasm_bindgen::to_value(&res).expect("failed to serialize result"))
}

/// Perform a regex replacement on a provided string
fn re_replace_impl(text: &str, reg_exp: &str, rep: &str, flags: &str) -> Result<JsValue, Error> {
    let (re, global) = re_build(reg_exp, flags)?;
    let text_bytes = text.as_bytes();
    let rep_bytes = rep.as_bytes();

    // Replace returns a Cow, get it as &str and turn into a js string
    if global {
        Ok(str::from_utf8(re.replace_all(text_bytes, rep_bytes).as_ref())?.into())
    } else {
        Ok(str::from_utf8(re.replace(text_bytes, rep_bytes).as_ref())?.into())
    }
}

/// Helper method to serialize our Result<...> type.
fn convert_res_to_jsvalue(res: Result<JsValue, Error>) -> JsValue {
    match res {
        Ok(v) => v,
        Err(e) => serde_wasm_bindgen::to_value(&e).expect("failed to serialize result"),
    }
}

/// Wrapper for `re_find_impl`
#[wasm_bindgen]
pub fn re_find(text: &str, reg_exp: &str, flags: &str) -> JsValue {
    convert_res_to_jsvalue(re_find_impl(text, reg_exp, flags))
}

/// Wrapper for `re_replace_impl`
#[wasm_bindgen]
pub fn re_replace(text: &str, reg_exp: &str, rep: &str, flags: &str) -> JsValue {
    convert_res_to_jsvalue(re_replace_impl(text, reg_exp, rep, flags))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_replace() {
        let res = re_replace("test 1234 end", r#"test (?P<cap>\d+)\s?"#, "$cap: ", "");
        assert_eq!(res, "1234: end");
    }
}
