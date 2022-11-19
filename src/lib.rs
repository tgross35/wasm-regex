use regex::{Regex, RegexBuilder};
// use serde::Serialize;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Process specified flags to create a regex query
/// Acceptable flags characters are `imsUux`
fn re_build(reg_exp: &str, flags: &str) -> Regex {
    let mut builder = RegexBuilder::new(reg_exp);
    let mut tmp_build = &mut builder;

    if flags.contains('i') {
        tmp_build = tmp_build.case_insensitive(true);
    }
    if flags.contains('m') {
        tmp_build = tmp_build.multi_line(true)
    }
    if flags.contains('s') {
        tmp_build = tmp_build.dot_matches_new_line(true)
    }
    if flags.contains('U') {
        tmp_build = tmp_build.swap_greed(true)
    }
    if flags.contains('u') {
        // Unicode is enabled by default, `u` disables
        tmp_build = tmp_build.unicode(false)
    }
    if flags.contains('x') {
        tmp_build = tmp_build.ignore_whitespace(true)
    }

    tmp_build.build().expect("failed to build regex")
}

/// Representation of all matches in some text
#[derive(Debug, Serialize)]
struct MatchSer {
    /// List of all matches
    matches: Vec<Vec<Option<CapSer>>>,
}

/// Representation of a single capture group
#[derive(Debug, Serialize)]
struct CapSer {
    /// Optional name of the capture group
    name: Option<String>,
    /// Whether or not this capture group represents the entire match (this will
    /// be the first capture group within its list)
    entire_match: bool,
    /// Content of the capture group
    content: String,
    /// Start index in the original string
    start: usize,
    /// End index in the original string
    end: usize,
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
pub fn re_find(text: &str, reg_exp: &str, flags: &str) -> String {
    let mut out = MatchSer {
        matches: Vec::new(),
    };

    let re = re_build(reg_exp, flags);

    for match_caps in re.captures_iter(text) {
        let mut match_vec: Vec<Option<CapSer>> = Vec::new();

        for (i, opt_cap_name) in re.capture_names().enumerate() {
            let match_ = match_caps.get(i).map(|m| CapSer {
                name: opt_cap_name.map(|n| n.to_owned()),
                entire_match: i == 0,
                content: m.as_str().to_owned(),
                start: m.start(),
                end: m.end(),
            });

            match_vec.push(match_);
        }

        out.matches.push(match_vec);
    }

    serde_json::to_string(&out).expect("failed to serialize regex")
}

/// Perform a regex replacement on a provided string
#[wasm_bindgen]
pub fn re_replace(text: &str, reg_exp: &str, rep: &str, flags: &str) -> String {
    let re = re_build(reg_exp, flags);
    re.replace(text, rep).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace() {
        let res = re_replace("test 1234 end", r#"test (?P<cap>\d+)\s?"#, "$cap: ", "");
        assert_eq!(res, "1234: end");
    }
}
