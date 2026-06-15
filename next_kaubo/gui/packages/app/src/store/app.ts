import { createSignal, batch } from "solid-js";
import type { AppStatus } from "@kaubo/types";
import { useKaubo } from "../hooks/useKaubo";

const DEFAULT_CODE = `// Kaubo Playground
var add = |a, b| {
    return a + b;
};
print(add(2, 3));
`;

export function createKauboStore() {
  const [code, setCode] = createSignal(DEFAULT_CODE);
  const [output, setOutput] = createSignal("");
  const [status, setStatus] = createSignal<AppStatus>("idle");
  const [error, setError] = createSignal<string | null>(null);
  const { doCompile, doRun, loading } = useKaubo();

  const compile = () => {
    batch(() => {
      setStatus("compiling");
      setOutput("");
      setError(null);
    });
    try {
      const len = doCompile(code());
      if (len == null) {
        setError("WASM not loaded yet");
        setStatus("idle");
        return;
      }
      batch(() => {
        setStatus("ready");
        setOutput(`Compiled: ${len} bytecodes\n`);
      });
    } catch (e: unknown) {
      batch(() => {
        setStatus("idle");
        setError(e instanceof Error ? e.message : String(e));
      });
    }
  };

  const run = () => {
    batch(() => {
      setStatus("running");
      setError(null);
    });
    // Yield a frame so the spinner can render before blocking WASM
    requestAnimationFrame(() => {
      try {
        const len = doCompile(code());
        if (len == null) {
          setError("WASM not loaded yet");
          setStatus("idle");
          return;
        }

        const out = doRun();
        if (out == null) {
          setError("WASM not loaded yet");
          setStatus("idle");
          return;
        }
        batch(() => {
          setStatus("ready");
          setOutput((prev) => prev + out);
        });
      } catch (e: unknown) {
        batch(() => {
          setStatus("ready");
          setError(e instanceof Error ? e.message : String(e));
        });
      }
    });
  };

  const clearError = () => setError(null);

  return { code, setCode, output, status, error, compile, run, clearError, loading };
}
