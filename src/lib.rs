//! Simple regex utility available via WASM

mod error;
use error::Error;
use regex::bytes::{Regex, RegexBuilder};
use serde::Serialize;
use std::str;
use wasm_bindgen::prelude::*;

/// Quick macro to print to the console for debugging
#[allow(unused_macros)]
macro_rules! console {
    ($($tt:tt)*) => {
        crate::log(&format!($($tt)*))
    };
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
#[allow(dead_code)]
fn log(s: &str) {
    eprintln!("{s}");
}

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
            if let Some(start) = cap_ser.start_utf8 {
                cap_ser.start = Some(find_idx(start));
            }
            if let Some(end) = cap_ser.end_utf8 {
                cap_ser.end = Some(find_idx(end));
            }
        }
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
    content: Option<Content<'a>>,
    /// Start index in the original string
    start: Option<usize>,
    /// Start index as a utf8 array
    start_utf8: Option<usize>,
    /// End index in the original string
    end: Option<usize>,
    /// End index as a utf8 array
    end_utf8: Option<usize>,
}

/// Our content is usually a string, but will be a byte slice if invalid utf8
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Content<'a> {
    String(&'a str),
    Bytes(u8),
}

impl<'a> Content<'a> {
    /// Return a sliced string if possible, byte array if not
    fn from_slice(text: &'a str, start: usize, end: usize) -> Self {
        if let Some(v) = text.get(start..end) {
            Self::String(v)
        } else {
            let sliced = &text.as_bytes()[start..end];
            // should only ever be used for single byte slices in non-unicode mode
            debug_assert_eq!(sliced.len(), 1);
            Self::Bytes(sliced[0])
        }
    }
}

///
#[derive(Debug)]
struct State {
    re: Regex,
    global: bool,
}

/// Process specified flags to create a regex query. Acceptable flags characters
/// are `gimsUux`. Also validates the regex string.
///
/// If the regex expression is empty, returns `None` for the state for short
/// circuiting
fn re_build(reg_exp: &str, flags: &str) -> Result<Option<State>, Error> {
    if reg_exp.is_empty() {
        return Ok(None);
    }

    // We keep a parser and builder separate; parser gives us nice errors,
    // builder creates the regex we need.
    let mut parser = regex_syntax::ParserBuilder::new();
    let mut builder = RegexBuilder::new(reg_exp);

    // Default to non unicode
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

    let _ = parser.build().parse(reg_exp)?;
    match builder.build() {
        Ok(re) => Ok(Some(State { re, global })),
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
    // We'll use this to convert our utf8 indices to utf16 in a more efficient way
    let mut all_indices: Vec<usize> = Vec::with_capacity(MATCH_ESTIMATE * 2);

    // Each item in this loop is a query match. Limit to `limit`.
    for (match_idx, cap_match) in re.captures_iter(text.as_bytes()).take(limit).enumerate() {
        // For each capture name, get the correct capture and turn it into a
        // serializable representation (CapSer). Collect it into a vector.
        let mut match_: Vec<CapSer> = Vec::with_capacity(re.captures_len());

        for (i, opt_cap_name) in re.capture_names().enumerate() {
            let mut to_push = CapSer {
                group_name: opt_cap_name,
                group_num: i,
                match_num: match_idx,
                ..CapSer::default()
            };

            // If our capture exists, update info for it
            if let Some(m) = cap_match.get(i) {
                let content = Content::from_slice(text, m.start(), m.end());
                all_indices.push(m.start());
                all_indices.push(m.end());
                to_push.is_participating = true;
                to_push.entire_match = i == 0;
                to_push.content = Some(content);
                to_push.start_utf8 = Some(m.start());
                to_push.end_utf8 = Some(m.end());
            }

            match_.push(to_push);
        }

        matches.push(match_);
    }

    let mut res = MatchSer { matches };

    // If we are matching on unicode, we need to update all byte indices to be
    // utf16 indices
    // if unicode {
    res.update_indices_utf16(text, all_indices);
    // }

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

    // Replace returns a Cow, get it as &str and turn into a js string
    if global {
        Ok(str::from_utf8(re.replace_all(text_bytes, rep_bytes).as_ref())?.into())
    } else {
        Ok(str::from_utf8(re.replace(text_bytes, rep_bytes).as_ref())?.into())
    }
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

    Ok(str::from_utf8(&dest)?.into())
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

/// Wrapper for `re_replace_list_impl`
#[wasm_bindgen]
pub fn re_replace_list(text: &str, reg_exp: &str, rep: &str, flags: &str) -> JsValue {
    convert_res_to_jsvalue(re_replace_list_impl(text, reg_exp, rep, flags))
}

/* helper functions */

/// Helper method to serialize our Result<...> type.
fn convert_res_to_jsvalue(res: Result<JsValue, Error>) -> JsValue {
    match res {
        Ok(v) => v,
        Err(e) => serde_wasm_bindgen::to_value(&e).expect("failed to serialize result"),
    }
}

/// Convert a utf8 **byte** index to utf16
fn utf16_index_bytes(s: &str, i: usize) -> usize {
    s[..i].chars().map(char::len_utf16).sum()
}

/// Take an unsorted list of `utf8_index` indices; sort them, update, and return
/// a map with the second element in each as the utf16 index
///
/// Panics if an index is outside of the string
fn utf16_index_bytes_slice(s: &str, mut indices: Vec<usize>) -> Vec<(usize, usize)> {
    // Sort by first element
    indices.sort_unstable();
    indices.dedup();
    let mut ret: Vec<(usize, usize)> = Vec::with_capacity(indices.len());

    // running total of the u16 string's length
    let mut total_u16_offset = 0usize;
    // Our iterator over indices to match
    let mut indices_iter = indices.iter().copied();
    // Our iterator over characters that could provide a match for our index
    let mut char_iter = s
        .char_indices()
        .map(|(byte_idx, ch)| (byte_idx, ch.len_utf8(), ch.len_utf16()))
        .map(|(byte_idx, ch8_len, ch16_len)| {
            let ret = (byte_idx, ch8_len, total_u16_offset);
            total_u16_offset += ch16_len;
            ret
        });

    // If we find a match that's not exact (for non-utf8 matches), save it for
    // reuse
    let mut residual_match: Option<(usize, usize)> = None;

    // Iterate through every index we need matched
    while let Some(idxu8) = indices_iter.next() {
        // Case 1: we are exactly at the end. Just push the current offset map
        // and quit
        if idxu8 == s.len() {
            ret.push((idxu8, total_u16_offset));
            break;
        }

        // Case 2: we have a stored value. This is used when we have an index
        // that is in between valid utf8 boundaries. Just push the cached value.
        if let Some((valid_until, last_u16_offset)) = residual_match {
            if idxu8 < valid_until {
                ret.push((idxu8, last_u16_offset));
                continue;
            }
        }

        // Case 3: We have a valid index and we can find it (=), or the next
        // valid index (>).
        let Some((byte_idx, ch8_len, u16_offset) )=
            char_iter.find(|(b_idx, _, _)| *b_idx >= idxu8) else {

            // Case 4: not found. If this is the case, we've hit the end of our
            // chars iterator. Just push the last known value for each remaining
            // index.
            ret.push((idxu8, total_u16_offset));
            indices_iter.for_each(|idxu8_inner| ret.push((idxu8_inner, total_u16_offset)));
            break;
        };

        ret.push((idxu8, u16_offset));

        // If we had a situation where we matched the next valid index instead
        // of our exact index, store that information for use in Case 1.
        if byte_idx > idxu8 {
            // If strictly greater, we will want to reuse this offset
            residual_match = Some((byte_idx + ch8_len, u16_offset));
        } else {
            residual_match = None;
        }
    }

    ret
}

/// Take a utf8 **char** index and convert it to utf16
fn utf16_index_chars(s: &str, i: usize) -> usize {
    s.chars().take(i).map(char::len_utf16).sum()
}

/// For debug, initialize the panic handler to print panics to the console
#[wasm_bindgen]
pub fn debug_init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[cfg(test)]
mod tests;
