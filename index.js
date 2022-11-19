let wasmRegex;
async function initWasm() {
    wasmRegex = await import('./pkg');
}
initWasm();

//Gets around an issue with Parcel
window.reFind = function reFind() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let flags = document.getElementById('flags').value;

    let output = wasmRegex.re_find(str, regExp, flags);
    console.log(output);

    document.getElementById('output').innerText = output;
}

window.reReplace = function reReplace() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let rep = document.getElementById('rep').value;
    let flags = document.getElementById('flags').value;

    let replaced = wasmRegex.re_replace(str, regExp, rep, flags);
    
    document.getElementById('output').innerText = replaced;
}
