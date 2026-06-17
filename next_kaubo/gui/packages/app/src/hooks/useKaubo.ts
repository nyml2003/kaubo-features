import { createResource } from "solid-js";
import init, { compile, run, diagnose } from "@kaubo/wasm";

export function useKaubo() {
  const [wasm] = createResource(async () => {
    await init();
    return { compile, run, diagnose };
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

  return { doCompile, doRun, doDiagnose, loading: () => wasm.loading };
}
