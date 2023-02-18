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
    /// List of all matches. The inner vector is a list of all groups.
    matches: Vec<Vec<CapSer<'a>>>,
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
    content: Option<&'a str>,
    /// Start index in the original string
    start: Option<usize>,
    /// End index in the original string
    end: Option<usize>,
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

    for flag in flags.chars() {
        match flag {
            'g' => global = true,
            'i' => builder_ref = builder_ref.case_insensitive(true),
            'm' => builder_ref = builder_ref.multi_line(true),
            's' => builder_ref = builder_ref.dot_matches_new_line(true),
            'U' => builder_ref = builder_ref.swap_greed(true),
            // Unicode is enabled by default, `u` disables
            'u' => builder_ref = builder_ref.unicode(false),
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

    // If we aren't global, limit to the first match
    let limit = if global { usize::MAX } else { 1 };
    let mut matches: Vec<Vec<CapSer>> = Vec::with_capacity(16);

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
                log(&format!(
                    "text: {text:?}\nstart: {}, end: {}",
                    m.start(),
                    m.end()
                ));
                let content = &text[m.start()..m.end()];
                to_push.is_participating = true;
                to_push.entire_match = i == 0;
                to_push.content = Some(content);
                to_push.start = Some(utf16_index_bytes(content, m.start()));
                to_push.end = Some(utf16_index_bytes(content, m.end()));
            }

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

/// Perform replacements and only return the matched string
fn re_replace_list_impl(
    text: &str,
    reg_exp: &str,
    rep: &str,
    flags: &str,
) -> Result<JsValue, Error> {
    let (re, global) = re_build(reg_exp, flags)?;
    let limit = if global { usize::MAX } else { 1 };
    let mut dest: Vec<u8> = Vec::with_capacity(text.len());

    // For each match, expand the replacement string and append it to our vector
    for cap_match in re.captures_iter(text.as_bytes()).take(limit) {
        cap_match.expand(rep.as_bytes(), &mut dest);
    }

    Ok(str::from_utf8(&dest)?.into())
}

/// Helper method to serialize our Result<...> type.
fn convert_res_to_jsvalue(res: Result<JsValue, Error>) -> JsValue {
    match res {
        Ok(v) => v,
        Err(e) => serde_wasm_bindgen::to_value(&e).expect("failed to serialize result"),
    }
}

/// Convert a utf8 **byte** index to utf16. We could do this more efficiently in a
/// batch probably, but it should be quick enough that we don't need to
fn utf16_index_bytes(s: &str, i: usize) -> usize {
    s[..i].chars().map(char::len_utf16).sum()
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

#[wasm_bindgen]
extern "C" {
    /// Log to the js console
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
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

#[cfg(test)]
mod tests {
    // tests marked wasm_bindgen_test must be run with `wasm-pack test --node` (not `cargo test`)
    use super::*;
    use wasm_bindgen_test::*;

    #[test]
    fn test_u16_index() {
        let s8 = "x😀🤣a🤩😛🏴‍☠️🤑";
        let s16: Vec<u16> = s8.encode_utf16().collect();

        // start index (u8), end (u8), expected value
        let to_test = [
            (0, 1, "x"),
            (1, 5, "😀"),
            (5, 14, "🤣a🤩"),
            (18, 31, "🏴‍☠️"),
            (31, 35, "🤑"),
        ];

        for (start8, end8, r8) in to_test.iter().copied() {
            let start16 = utf16_index_bytes(s8, start8);
            let end16 = utf16_index_bytes(s8, end8);
            let r16: Vec<u16> = r8.encode_utf16().collect();

            assert_eq!(&s8[start8..end8], r8);
            assert_eq!(&s16[start16..end16], r16);
        }
    }

    // #[wasm_bindgen_test]
    // fn test_find_unicode() {
    //     let res = re_find("😃", ".", "");
    //     // dbg!(&res);
    //     assert_eq!(res, "1234: end");
    //     assert_eq!(res.as_string().unwrap(), "1234: end");
    // }

    #[wasm_bindgen_test]
    fn test_replace() {
        let res = re_replace("test 1234 end", r#"test (?P<cap>\d+)\s?"#, "$cap: ", "");
        assert_eq!(res.as_string().unwrap(), "1234: end");
    }

    #[wasm_bindgen_test]
    fn test_replace_list() {
        let res = re_replace_list("foo bar!", r#"\w+"#, "$0\n", "g");
        assert_eq!(res.as_string().unwrap(), "foo\nbar\n");
    }
}
