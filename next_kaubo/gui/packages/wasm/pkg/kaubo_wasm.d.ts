/* tslint:disable */
/* eslint-disable */

/**
 * Compile Kaubo source code, store chunk in memory.
 * Returns number of bytecode instructions (for display).
 */
export function compile(source: string): number;

/**
 * Initialize panic hook so errors show in browser console instead of `unreachable`
 */
export function init(): void;

/**
 * Tokenize Kaubo source and return a JSON array of tokens.
 *
 * Each token: `{"kind":"keyword","from":0,"to":3}`
 * Positions are UTF-16 code unit offsets (compatible with JavaScript / CodeMirror).
 */
export function lex(source: string): string;

/**
 * Run the most recently compiled chunk, returns stdout output
 */
export function run(_bytes: Uint8Array): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile: (a: number, b: number) => [number, number, number];
    readonly init: () => void;
    readonly lex: (a: number, b: number) => [number, number];
    readonly run: (a: number, b: number) => [number, number, number, number];
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
