import init, { compile, diagnose, format, lsp_on_change, run } from "@kaubo/wasm";
import { createResource } from "solid-js";

export function useKaubo() {
  const [wasm] = createResource(async () => {
    await init();
    return { compile, run, diagnose, format, lsp_on_change };
  });

  const doCompile = (source: string): number => {
    const w = wasm();
    if (!w) throw new Error("WASM not loaded");
    return w.compile(source);
  };

  const doRun = (): string => {
    const w = wasm();
    if (!w) throw new Error("WASM not loaded");
    return w.run(new Uint8Array());
  };

  const doDiagnose = (source: string): string => {
    const w = wasm();
    if (!w) throw new Error("WASM not loaded");
    return w.diagnose(source);
  };

  const doLspOnChange = (source: string): void => {
    const w = wasm();
    if (!w) throw new Error("WASM not loaded");
    w.lsp_on_change(source);
  };

  const doFormat = (source: string): string => {
    const w = wasm();
    if (!w) throw new Error("WASM not loaded");
    const result = w.format(source);
    if (result.startsWith("// format error:")) {
      throw new Error(result.slice(16));
    }
    return result;
  };

  return { doCompile, doRun, doDiagnose, doFormat, doLspOnChange, loading: () => wasm.loading };
}
