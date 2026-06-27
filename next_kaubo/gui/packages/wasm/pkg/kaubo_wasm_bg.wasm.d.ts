/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const init: () => void;
export const lex: (a: number, b: number) => [number, number];
export const diagnose: (a: number, b: number) => [number, number];
export const set_log_level: (a: number) => void;
export const compile: (a: number, b: number) => [number, number, number];
export const run: (a: number, b: number) => [number, number, number, number];
export const hover: (a: number, b: number, c: number) => [number, number];
export const semantic_tokens: (a: number, b: number) => [number, number];
export const complete: (a: number, b: number, c: number) => [number, number];
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_start: () => void;
