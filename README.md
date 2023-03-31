# wasm-regex

A very rudimentary demo of Rust's regex capabilities used in WASM. Follows
[Mozilla's guide](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm).

## Running

First, if you don't already have it, install [wasm-pack](https://github.com/rustwasm/wasm-pack)
with `cargo install wasm-pack`.

Next, inside the main folder, run `wasm-pack build` (add `--release` for full optimization).

Add the flag `--featurues js-console` to enable printing to the console, for debugging

To build an even smaller wasm file (for releases), use:
`wasm-pack build --release --no-typescript --features none -Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort`.

Install the necessary packages with `npm install`.

Finally, run `npm run serve` to get the site up and going locally.


## API

Function signatures:

```js
function re_find(
    text: string, reg_exp: string, flags: string,
    text_sep?: string, reg_exp_sep?: string): string;

function re_replace(
    text: string, reg_exp: string, rep: string, flags: string,
    text_sep?: string, reg_exp_sep?: string, rep_sep?: string): string;

function re_replace_list(
    text: string, reg_exp: string, rep: string, flags: string,
    text_sep?: string, reg_exp_sep?: string, rep_sep?: string): string;
```

The `_sep` parameters are optional and indicate how this library should parse
the string. Options are `ignore`, `str`, `raw`, `rawhash1`, `rawhash2`,
`rawhash3`, or `rawhash4`, to parse like the contents of `foo`, `"foo"`,
`r"foo"`, `r#"foo"#`, `r##"foo"##`, `r###"foo"###`, `r####"foo"####`,
respectively (quotes shouldn't be included). If unspecified, this defaults to
`ignore`, which does no extra escaping.

Result of `re_find`:

```json5
{
    // This is the only key in the JSON. Each item in this array represents a
    // single match
    "matches": [ 
        // Each item within this array represents a single group
        [
            {
                // Index of the group
                "groupNum": 0,
                // Only present if the group is named
                "groupName": "a",
                // Index of the match
                "match": 0,
                // For optional capture groups, whether or 
                "isParticipating": true,
                // Whether or not this represents the entire match (true for
                // groupNum 0)
                "entireMatch": true,

                /* the below fields only exist if isParticipating is true */

                // Exact content of the match. If the unicode flag (`u`) is not present
                // and the match contains an invalid unicode char, this will be a
                // byte slice instead (numbers of that byte in the utf8 match)
                "content": "ab",
                // Position of the match in UTF-8. Use for user-displayed offsets
                "start": 0,
                "end": 2,
                // Position of the match in UTF-16. Use for highlighting in JS
                "startUtf16": 0,
                "endUtf16": 2,
            },
        ],
    ]
}
```

Result of `re_replace` is just a string with all replacements applied. Result of
`re_replace_list` is a string with replacements applied to each match, without
any non-matching characters.

### Error result

Error results have two keys: `error_class` indicating the type of error, and
`error` giving the contents.

`unescape` errors happen when there is an issue quoting a given string.

```json5
{
    "errorClass": "unescape",
    "error": {
        "kind": "Pattern",
        "message": "pattern '\"#' may not be contained in r#\" strings",
        "span": {
            "start": { "offset": 0, "line": 1, "column": 0 },
            "end": { "offset": 2, "line": 2, "column": 2 }
        },
        "span_utf16": {
            "start": { "offset": 0, "line": 1, "column": 0 },
            "end": { "offset": 2, "line": 2, "column": 2 }
        },
        // "text", "reg_exp", or "rep" based on which field caused the error
        "source": "text"
    }
}
```


`regexSyntax` is the main error type, which is an error with the given syntax.
It can be tested with something like the regex query `)`.

```json5
{
    "errorClass": "regexSyntax",
    "error": {
        // Identifier of the error kind from `regex_syntax`
        "kind": "GroupUnopened",
        // Message provided by parser about the error
        "message": "unopened group",
        // Offending pattern
        "pattern": ")",
        // Location of the error in the pattern in utf8, this is just for
        // debugging purposes
        "span": {
            "start": { "offset": 0, "line": 1, "column": 1 },
            "end": { "offset": 1, "line": 1, "column": 2 }
        },
        // Use this span to indicate position on the JS side
        "span_utf16": {
            "start": { "offset": 0, "line": 1, "column": 1 },
            "end": { "offset": 1, "line": 1, "column": 2 }
        }
    }
}
```

`regexCompiledTooBig` (exceeds compile size limit), `regexUnspecified`
(unspecified error - never expected to happen), are the two remaining error
types, and they are should be pretty unlikely.

```json5
{
    "errorClass": "regexCompiledTooBig",
    "error": "Compiled regex exceeds size limit of 0 bytes."
}
```

(in a previous version, there was an `encoding` error, but now it just does a
lossy UTF-8 encoding instead).
