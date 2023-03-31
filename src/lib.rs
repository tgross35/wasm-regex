//! Simple regex utility available via WASM

mod error;
mod strops;
mod util;

use std::borrow::Cow;
use std::str;

use error::Error;
use regex::bytes::{Regex, RegexBuilder};
use serde::Serialize;
use strops::{str_from_utf8_rep, unescape, utf16_index_bytes_slice};
use wasm_bindgen::prelude::*;

/// Representation of all matches in some text
#[derive(Debug, Serialize, Default)]
#[serde(rename_all(serialize = "camelCase"))]
struct MatchSer<'a> {
    /// List of all matches. The inner vector is a list of all groups.
    matches: Vec<Vec<CapSer<'a>>>,
}

impl<'a> MatchSer<'a> {
    /// Serialize myself
    fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self).expect("failed to serialize result")
    }

    /// For all matches, set indices to utf16 for the given text
    fn update_indices_utf16(&mut self, text: &str, indices: Vec<usize>) {
        // Get our indices from the text
        let matched_indices = utf16_index_bytes_slice(text, indices);

        // convenience closure; find the correct element by binary search
        let find_idx = |search| {
            matched_indices[matched_indices
                .binary_search_by_key(&search, |(idxu8, _)| *idxu8)
                .unwrap()]
            .1
        };

        for cap_ser in self.matches.iter_mut().flatten() {
            if let Some(start) = cap_ser.start {
                cap_ser.start_utf16 = Some(find_idx(start));
            }
            if let Some(end) = cap_ser.end {
                cap_ser.end_utf16 = Some(find_idx(end));
            }
        }
    }
}

/// Result of a replacement. The purpose of this struct is just to wrap the
/// string within a "result" key for the JS result.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all(serialize = "camelCase"))]
struct ReplacdSer<'a> {
    result: &'a str,
}

impl<'a> ReplacdSer<'a> {
    /// Serialize myself
    fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self).expect("failed to serialize result")
    }
}

/// Representation of a single capture group
#[derive(Debug, Serialize, Default)]
#[serde(rename_all(serialize = "camelCase"))]
struct CapSer<'a> {
    /// Optional name of the capture group
    group_name: Option<&'a str>,
    /// Index of the match within all matches
    #[serde(rename = "match")]
    match_num: usize,
    /// Index of the group within this single match
    group_num: usize,
    /// Whether or not an optional group is found within the match
    is_participating: bool,
    /// Whether or not this capture group represents the entire match (this will
    /// be the first capture group within its list)
    entire_match: bool,

    /* below fields only exist if is_participating */
    /// Content of the capture group
    content: Option<Cow<'a, str>>,
    /// Start index in the original string
    start_utf16: Option<usize>,
    /// Start index as a utf8 array
    start: Option<usize>,
    /// End index in the original string
    end_utf16: Option<usize>,
    /// End index as a utf8 array
    end: Option<usize>,
}

/// Our regex state with compiled regex and global flag
#[derive(Debug)]
struct State {
    re: Regex,
    global: bool,
}

/// Process specified flags to create a regex query. Acceptable flags characters
/// are `gimsUux`. Also validates the regex string.
///
/// If the regex expression is empty, returns `None` for the state, allowing for
/// short circuiting
fn re_build(reg_exp: &str, flags: &str) -> Result<Option<State>, Error> {
    if reg_exp.is_empty() {
        return Ok(None);
    }

    // We keep a parser and builder separate; parser gives us nice errors,
    // builder creates the regex we need.
    let mut parser = regex_syntax::ParserBuilder::new();
    let mut builder = RegexBuilder::new(reg_exp);

    // Default to non-unicode, non-global
    let mut global = false;
    parser.allow_invalid_utf8(true);
    parser.unicode(false);
    builder.unicode(false);

    for flag in flags.chars() {
        // We need to apply all flags to both our builder and our parser
        match flag {
            'g' => global = true,
            'i' => {
                builder.case_insensitive(true);
                parser.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
                parser.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
                parser.dot_matches_new_line(true);
            }
            'U' => {
                builder.swap_greed(true);
                parser.swap_greed(true);
            }
            'u' => {
                builder.unicode(true);
                parser.unicode(true);
            }
            'x' => {
                builder.ignore_whitespace(true);
                parser.ignore_whitespace(true);
            }
            // We can panic here because the UI should only ever give us valid
            // flags
            _ => panic!("unrecognized flag"),
        }
    }

    // Create nice errors
    let _ = parser.build().parse(reg_exp)?;

    // Build our pattern
    match builder.build() {
        Ok(re) => Ok(Some(State { re, global })),
        Err(e) => Err(e.into()),
    }
}

/// Run a regular expression on a block of text, returning a JSON string
///
/// # Arguments
///
/// - `flags`: apply global flags, options `gimsUux`
/// - `text`: haystack to search in
/// - `reg_exp`: regular expression to match against
///
/// Returns a string JSON representation of `CapSer`
fn re_find_impl(text: &str, reg_exp: &str, flags: &str) -> Result<JsValue, Error> {
    const MATCH_ESTIMATE: usize = 16; // estimate for vec size initialization

    let Some(State {
        re,
        global,
    }) = re_build(reg_exp, flags)? else {
        return Ok(MatchSer::default().to_js_value());
    };

    // If we aren't global, limit to the first match
    let limit = if global { usize::MAX } else { 1 };
    let mut matches: Vec<Vec<CapSer>> = Vec::with_capacity(MATCH_ESTIMATE);
    // We'll use this to convert our utf8 indices to utf16 all at once
    let mut all_indices: Vec<usize> = Vec::with_capacity(MATCH_ESTIMATE * 2);

    // Each item in this loop is a query match. Limit to `limit`.
    for (match_idx, cap_match) in re.captures_iter(text.as_bytes()).take(limit).enumerate() {
        // For each capture name, get the correct capture and turn it into a
        // serializable representation (CapSer). Collect it into a vector.
        let mut match_: Vec<CapSer> = Vec::with_capacity(re.captures_len());

        for (i, opt_cap_name) in re.capture_names().enumerate() {
            // Start with a default capture representation
            let mut to_push = CapSer {
                group_name: opt_cap_name,
                group_num: i,
                match_num: match_idx,
                ..CapSer::default()
            };

            // If our capture exists, update info for it
            if let Some(m) = cap_match.get(i) {
                let content = str_from_utf8_rep(text, m.start(), m.end());

                all_indices.push(m.start());
                all_indices.push(m.end());

                to_push.is_participating = true;
                to_push.entire_match = i == 0;
                to_push.content = Some(content);
                to_push.start = Some(m.start());
                to_push.end = Some(m.end());
            }

            match_.push(to_push);
        }

        matches.push(match_);
    }

    let mut res = MatchSer { matches };

    // We need to add valid utf16 indices, for js highlighting
    res.update_indices_utf16(text, all_indices);

    Ok(res.to_js_value())
}

/// Perform a regex replacement on a provided string
fn re_replace_impl(text: &str, reg_exp: &str, rep: &str, flags: &str) -> Result<JsValue, Error> {
    let Some(State {
        re,
        global,
    }) = re_build(reg_exp, flags)?  else {
        return Ok(text.into());
    };

    let text_bytes = text.as_bytes();
    let rep_bytes = rep.as_bytes();

    let res_cow = if global {
        re.replace_all(text_bytes, rep_bytes)
    } else {
        re.replace(text_bytes, rep_bytes)
    };

    // Replace returns a Cow, get it as &str and turn into a js string
    // Invalid unicode is replaced with the invalid unicode character
    let rep_ser = ReplacdSer {
        result: &String::from_utf8_lossy(res_cow.as_ref()),
    };
    Ok(rep_ser.to_js_value())
}

/// Perform replacements and only return the matched string
fn re_replace_list_impl(
    text: &str,
    reg_exp: &str,
    rep: &str,
    flags: &str,
) -> Result<JsValue, Error> {
    let Some(State {
        re,
        global,
    }) = re_build(reg_exp, flags)?  else {
        return Ok("".into());
    };

    let limit = if global { usize::MAX } else { 1 };
    let mut dest: Vec<u8> = Vec::with_capacity(text.len());

    // For each match, expand the replacement string and append it to our vector
    for cap_match in re.captures_iter(text.as_bytes()).take(limit) {
        cap_match.expand(rep.as_bytes(), &mut dest);
    }

    // Return a valid utf8 string that uses the replacement character where needed
    let rep_ser = ReplacdSer {
        result: &String::from_utf8_lossy(&dest),
    };

    Ok(rep_ser.to_js_value())
}

/// Wrapper for `re_find_impl`
#[wasm_bindgen]
pub fn re_find(
    text: &str,
    reg_exp: &str,
    flags: &str,
    text_sep: Option<String>,
    reg_exp_sep: Option<String>,
) -> JsValue {
    wrap_erroring_fn(|| {
        let text_esc = unescape(text, &text_sep).map_err(|e| (e, "text"))?;
        let reg_exp_esc = unescape(reg_exp, &reg_exp_sep).map_err(|e| (e, "reg_exp"))?;
        re_find_impl(&text_esc, &reg_exp_esc, flags)
    })
}

/// Wrapper for `re_replace_impl`
#[wasm_bindgen]
pub fn re_replace(
    text: &str,
    reg_exp: &str,
    rep: &str,
    flags: &str,
    text_sep: Option<String>,
    reg_exp_sep: Option<String>,
    rep_sep: Option<String>,
) -> JsValue {
    wrap_erroring_fn(|| {
        let text_esc = unescape(text, &text_sep).map_err(|e| (e, "text"))?;
        let reg_exp_esc = unescape(reg_exp, &reg_exp_sep).map_err(|e| (e, "reg_exp"))?;
        let rep_esc = unescape(rep, &rep_sep).map_err(|e| (e, "rep"))?;
        re_replace_impl(&text_esc, &reg_exp_esc, &rep_esc, flags)
    })
}

/// Wrapper for `re_replace_list_impl`
#[wasm_bindgen]
pub fn re_replace_list(
    text: &str,
    reg_exp: &str,
    rep: &str,
    flags: &str,
    text_sep: Option<String>,
    reg_exp_sep: Option<String>,
    rep_sep: Option<String>,
) -> JsValue {
    wrap_erroring_fn(|| {
        let text_esc = unescape(text, &text_sep).map_err(|e| (e, "text"))?;
        let reg_exp_esc = unescape(reg_exp, &reg_exp_sep).map_err(|e| (e, "reg_exp"))?;
        let rep_exc = unescape(rep, &rep_sep).map_err(|e| (e, "rep"))?;
        re_replace_list_impl(&text_esc, &reg_exp_esc, &rep_exc, flags)
    })
}

/* helper functions */

/// Helper method that lets us use `?` to propegate errors, and serializes
/// everything to a `JsValue`
fn wrap_erroring_fn<F>(f: F) -> JsValue
where
    F: FnOnce() -> Result<JsValue, Error>,
{
    match f() {
        Ok(v) => v,
        Err(e) => serde_wasm_bindgen::to_value(&e).expect("failed to serialize result"),
    }
}

#[cfg(test)]
mod tests;
