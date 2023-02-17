use regex::{Regex, RegexBuilder};
use serde::Serialize;
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
    /// Index of the group
    group_num: usize,
    /// Index of the match
    #[serde(rename = "match")]
    match_num: usize,
    /// Whether or not this capture group represents the entire match (this will
    /// be the first capture group within its list)
    entire_match: bool,
    /// Content of the capture group
    content: &'a str,
    /// Start index in the original string
    start: usize,
    /// End index in the original string
    end: usize,
}

/// Wrapper so we can serialize regex errors
#[derive(Debug, Serialize)]
struct Error {
    error: String,
}

/// Allow automatic conversion from error to a JS string with `?`
impl From<Error> for JsValue {
    fn from(value: Error) -> Self {
        serde_wasm_bindgen::to_value(&value).expect("failed to serialize regex")
    }
}

/// Process specified flags to create a regex query. Acceptable flags characters
/// are `gimsUux`
///
/// The returned bool indicates if global
fn re_build(reg_exp: &str, flags: &str) -> Result<(Regex, bool), Error> {
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
            _ => panic!("unrecognized flag"),
        }
    }

    match builder.build() {
        Ok(re) => Ok((re, global)),
        Err(e) => Err(Error {
            error: e.to_string(),
        }),
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
#[wasm_bindgen]
pub fn re_find(text: &str, reg_exp: &str, flags: &str) -> JsValue {
    let built_re = re_build(reg_exp, flags);
    let Ok((re, global)) = built_re else {
        // Handle error
        return built_re.unwrap_err().into();
    };
    let default_cap = if global { re.captures_len() } else { 1 };
    let limit = if global { usize::MAX } else { 1 };
    let mut matches: Vec<Vec<Option<CapSer>>> = Vec::with_capacity(default_cap);

    // If we aren't global, limit to the first match

    // Each item in this loop is a query match. Limit to `limit`.
    for (match_idx, cap_match) in re.captures_iter(text).take(limit).enumerate() {
        // For each capture name, get the correct capture and turn it into a
        // serializable representation (CapSer). Collect it into a vector.
        let match_: Vec<Option<CapSer>> = re
            .capture_names()
            .enumerate()
            .map(|(i, opt_cap_name)| {
                cap_match.get(i).map(|m| CapSer {
                    group_name: opt_cap_name,
                    group_num: i,
                    match_num: match_idx,
                    entire_match: i == 0,
                    content: m.as_str(),
                    start: m.start(),
                    end: m.end(),
                })
            })
            .collect();

        matches.push(match_);
    }

    let out = MatchSer { matches };

    serde_wasm_bindgen::to_value(&out).expect("failed to serialize regex")
}

/// Perform a regex replacement on a provided string
#[wasm_bindgen]
pub fn re_replace(text: &str, reg_exp: &str, rep: &str, flags: &str) -> JsValue {
    let built_re = re_build(reg_exp, flags);
    let Ok((re, global)) = built_re else {
        return built_re.unwrap_err().into();
    };

    // Replace returns a Cow, get it as &str and turn into a js string
    if global {
        re.replace_all(text, rep).as_ref().into()
    } else {
        re.replace(text, rep).as_ref().into()
    }
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
