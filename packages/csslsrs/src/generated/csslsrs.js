
let imports = {};
import * as import0 from './csslsrs_bg.js';
imports['./csslsrs_bg.js'] = import0;

import * as path from 'node:path';
import * as fs from 'node:fs';
import * as process from 'node:process';

let file = path.dirname(new URL(import.meta.url).pathname);
if (process.platform === 'win32') {
    file = file.substring(1);
}
const bytes = fs.readFileSync(path.join(file, 'csslsrs_bg.wasm'));

const wasmModule = new WebAssembly.Module(bytes);
const wasmInstance = new WebAssembly.Instance(wasmModule, imports);
const wasm = wasmInstance.exports;
export const __wasm = wasm;

imports["./csslsrs_bg.js"].__wbg_set_wasm(wasm);
export * from "./csslsrs_bg.js";