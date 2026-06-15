import { createResource } from "solid-js";
import init, { compile, run, diagnose } from "@kaubo/wasm";

export function useKaubo() {
  const [wasm] = createResource(async () => {
    await init();
    return { compile, run, diagnose };
  });

  const doCompile = (source: string): number | null => {
    const w = wasm();
    return w ? w.compile(source) : null;
  };

  const doRun = (): string | null => {
    const w = wasm();
    return w ? w.run(new Uint8Array()) : null;
  };

  const doDiagnose = (source: string): string | null => {
    const w = wasm();
    return w ? w.diagnose(source) : null;
  };

  return { doCompile, doRun, doDiagnose, loading: () => wasm.loading };
}
