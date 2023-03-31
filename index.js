let wasmRegex;
async function initWasm() {
    wasmRegex = await import('./pkg');
    // enable if the console handler is also enabled on the rust side
    wasmRegex.debug_init();
}
initWasm();

// Wrappers for debugging
window.re_find = function re_find(text, pat, flags, text_esc, pat_esc) {
    return wasmRegex.re_find(text, pat, flags, text_esc, pat_esc);
}

window.re_replace = function re_find(text, pat, rep, flags, text_esc, pat_esc, rep_esc) {
    return wasmRegex.re_replace(text, pat, rep, flags,text_esc, pat_esc, rep_esc);
}

window.re_replace_list = function re_find(text, pat, rep, flags) {
    return wasmRegex.re_replace_list(text, pat, rep, flags, text_esc, pat_esc, rep_esc);
}

window.reFind = function reFind() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let flags = document.getElementById('flags').value;
    let textSep = document.getElementById('textSep').value;
    let regExpSep = document.getElementById('regExpSep').value;

    let start = new Date().getTime();
    let output = wasmRegex.re_find(str, regExp, flags, textSep, regExpSep);
    let end = new Date().getTime();

    document.getElementById('output').innerText = JSON.stringify(output, null, 4);
    document.getElementById('operation_time').innerText = end - start;
    document.getElementById('input_len').innerText = str.length;
}

window.reReplace = function reReplace() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let rep = document.getElementById('rep').value;
    let flags = document.getElementById('flags').value;
    let textSep = document.getElementById('textSep').value;
    let regExpSep = document.getElementById('regExpSep').value;
    let repSep = document.getElementById('repSep').value;

    let start = new Date().getTime();
    let output = wasmRegex.re_replace(str, regExp, rep, flags, textSep, regExpSep, repSep);
    let end = new Date().getTime();
    
    document.getElementById('output').innerText = JSON.stringify(output, null, 4);
    document.getElementById('operation_time').innerText = end - start;
    document.getElementById('input_len').innerText = str.length;
}

window.reReplaceList = function reReplace() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let rep = document.getElementById('rep').value;
    let flags = document.getElementById('flags').value;
    let textSep = document.getElementById('textSep').value;
    let regExpSep = document.getElementById('regExpSep').value;
    let repSep = document.getElementById('repSep').value;

    let start = new Date().getTime();
    let output = wasmRegex.re_replace_list(str, regExp, rep, flags, textSep, regExpSep, repSep);
    let end = new Date().getTime();
    
    document.getElementById('output').innerText = JSON.stringify(output, null, 4);
    document.getElementById('operation_time').innerText = end - start;
    document.getElementById('input_len').innerText = str.length;
}
