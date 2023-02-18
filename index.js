let wasmRegex;
async function initWasm() {
    wasmRegex = await import('./pkg');
    wasmRegex.debug_init();
}
initWasm();

window.reFind = function reFind() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let flags = document.getElementById('flags').value;

    let start = new Date().getTime();
    let output = wasmRegex.re_find(str, regExp, flags);
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

    let start = new Date().getTime();
    let replaced = wasmRegex.re_replace(str, regExp, rep, flags);
    let end = new Date().getTime();
    
    document.getElementById('output').innerText = replaced;
    document.getElementById('operation_time').innerText = end - start;
    document.getElementById('input_len').innerText = str.length;
}

window.reReplaceList = function reReplace() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let rep = document.getElementById('rep').value;
    let flags = document.getElementById('flags').value;

    let start = new Date().getTime();
    let replaced = wasmRegex.re_replace_list(str, regExp, rep, flags);
    let end = new Date().getTime();
    
    document.getElementById('output').innerText = replaced;
    document.getElementById('operation_time').innerText = end - start;
    document.getElementById('input_len').innerText = str.length;
}
