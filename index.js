let wasmRegex;
async function initWasm() {
    wasmRegex = await import('./pkg');
}
initWasm();

//Gets around an issue with Parcel
window.reMatches = function reMatches() {
    let str = document.getElementById('str').value;
    let regExp = document.getElementById('regExp').value;
    let flags = document.getElementById('flags').value;

    let output = wasmRegex.re_matches(str, regExp, flags);
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
