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


## API

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

                // Exact content of the match
                "content": "ab",
                // Start index of the match in UTF-8
                "start": 0,
                // End index of the match in UTF-8
                "end": 2
            },
        ],
    ]
}
```

Result of `re_replace` is just a string with all replacements applied.

Result of `re_replace_list` is a string with replacements applied to each match,
without any non-matching characters.

### Error result

Error results have two keys: `error_class` indicating the type of error, and
`error` giving the contents.

`regexSyntax` is the main error type, which is an error with the given syntax.

```json5
{
    "error_class": "regexSyntax",
    "error": {
        // Identifier of the error kind from `regex_syntax`
        "kind": "GroupUnopened",
        // Message provided by parser about the error
        "message": "unopened group",
        // Offending pattern
        "pattern": ")",
        // Location of the error in the pattern
        "span": {
            "start": {
                "offset": 0,
                "line": 1,
                "column": 1
            },
            "end": {
                "offset": 1,
                "line": 1,
                "column": 2
            }
        }
    }
}
```

`regexCompiledTooBig` (exceeds compile size limit), `regexUnspecified`
(unspecified error - never expected to happen), and `encoding` (something weird
happened at a UTF-8 boundary) are the three remaining error types, and they are
all pretty unlikely.

```json5
{
    "error_class": "regexCompiledTooBig",
    "error": "Compiled regex exceeds size limit of 0 bytes."
}
```
