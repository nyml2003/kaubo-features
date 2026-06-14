import { createResource } from "solid-js";
import init, { compile, run } from "@kaubo/wasm";

export function useKaubo() {
  const [wasm] = createResource(async () => {
    await init();
    return { compile, run };
  });

  const doCompile = (source: string): number | null => {
    const w = wasm();
    return w ? w.compile(source) : null;
  };

  const doRun = (): string | null => {
    const w = wasm();
    return w ? w.run(new Uint8Array()) : null;
  };

  return { doCompile, doRun, loading: () => wasm.loading };
}
