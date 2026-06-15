import { createSignal, batch, onCleanup } from "solid-js";
import type { AppStatus } from "@kaubo/types";
import { useKaubo } from "../hooks/useKaubo";
import { setKauboDiagnostics, type KauboError } from "../editor/kauboLang";
import type { ThemeName } from "../themes";
import type { KauboExample } from "../examples";

const DEFAULT_CODE = `// Kaubo Playground
var add = |a, b| {
    return a + b;
};
print(add(2, 3));
`;

const DIAGNOSE_DEBOUNCE_MS = 400;

function getStoredTheme(): ThemeName {
  try {
    const stored = localStorage.getItem("kaubo-theme");
    if (stored === "material-dark" || stored === "nord" || stored === "gruvbox-dark" || stored === "min-light" || stored === "high-contrast") {
      return stored;
    }
  } catch {
    // localStorage unavailable
  }
  return "material-dark";
}

export function createKauboStore() {
  const [code, setCode] = createSignal(DEFAULT_CODE);
  const [output, setOutput] = createSignal("");
  const [status, setStatus] = createSignal<AppStatus>("idle");
  const [error, setError] = createSignal<string | null>(null);
  const [theme, setThemeSignal] = createSignal<ThemeName>(getStoredTheme());
  const [activeExample, setActiveExample] = createSignal<string | null>(null);
  const [examplesExpanded, setExamplesExpanded] = createSignal(true);
  const { doCompile, doRun, doDiagnose, loading } = useKaubo();

  let diagnoseTimer: ReturnType<typeof setTimeout> | null = null;

  const setTheme = (name: ThemeName) => {
    setThemeSignal(name);
    try { localStorage.setItem("kaubo-theme", name); } catch { /* noop */ }
  };

  const toggleExamples = () => setExamplesExpanded((prev) => !prev);

  const loadExample = (ex: KauboExample) => {
    setActiveExample(ex.id);
    setCode(ex.code);
    setOutput("");
    setError(null);
    setKauboDiagnostics(null);
    if (diagnoseTimer) clearTimeout(diagnoseTimer);
  };

  function runDiagnose(source: string) {
    try {
      const json = doDiagnose(source);
      if (json == null) return;
      const parsed: KauboError[] = JSON.parse(json) as KauboError[];
      setKauboDiagnostics(parsed);
    } catch {
      setKauboDiagnostics(null);
    }
  }

  function scheduleDiagnose(source: string) {
    if (diagnoseTimer) clearTimeout(diagnoseTimer);
    diagnoseTimer = setTimeout(() => { runDiagnose(source); }, DIAGNOSE_DEBOUNCE_MS);
  }

  onCleanup(() => {
    if (diagnoseTimer) clearTimeout(diagnoseTimer);
  });

  const updateCode = (newCode: string) => {
    setCode(newCode);
    setActiveExample(null);
    scheduleDiagnose(newCode);
  };

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
        setOutput(`Compiled: ${String(len)} bytecodes\n`);
        setKauboDiagnostics(null);
      });
    } catch (e: unknown) {
      batch(() => {
        setStatus("idle");
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
      });
      runDiagnose(code());
    }
  };

  const run = () => {
    batch(() => {
      setStatus("running");
      setError(null);
    });
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
          setKauboDiagnostics(null);
        });
      } catch (e: unknown) {
        batch(() => {
          setStatus("ready");
          setError(e instanceof Error ? e.message : String(e));
        });
        runDiagnose(code());
      }
    });
  };

  const clearError = () => {
    setError(null);
    setKauboDiagnostics(null);
  };

  return {
    code, setCode: updateCode, output, status, error,
    theme, setTheme,
    activeExample, examplesExpanded, toggleExamples, loadExample,
    compile, run, clearError, loading,
  };
}
