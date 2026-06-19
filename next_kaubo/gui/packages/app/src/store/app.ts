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
const WASM_NOT_LOADED_MESSAGE = "WASM not loaded yet";

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

function getStoredTabSize(): number {
  try {
    const stored = localStorage.getItem("kaubo-tabsize");
    if (stored === "2" || stored === "4") {
      return parseInt(stored);
    }
  } catch {
    // localStorage unavailable
  }
  return 4;
}

function getStoredFontSize(): number {
  try {
    const stored = localStorage.getItem("kaubo-fontsize");
    if (stored) {
      const n = parseInt(stored);
      if (n >= 10 && n <= 24) return n;
    }
  } catch {
    // localStorage unavailable
  }
  return 14;
}

export function createKauboStore() {
  const [code, setCode] = createSignal(DEFAULT_CODE);
  const [output, setOutput] = createSignal("");
  const [status, setStatus] = createSignal<AppStatus>("idle");
  const [error, setError] = createSignal<string | null>(null);
  const [theme, setThemeSignal] = createSignal<ThemeName>(getStoredTheme());
  const [activeExample, setActiveExample] = createSignal<string | null>(null);
  const [examplesExpanded, setExamplesExpanded] = createSignal(true);
  const [tabSize, setTabSizeSignal] = createSignal<number>(getStoredTabSize());
  const [fontSize, setFontSizeSignal] = createSignal<number>(getStoredFontSize());
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const { doCompile, doRun, doDiagnose, loading } = useKaubo();

  let diagnoseTimer: ReturnType<typeof setTimeout> | null = null;

  const setTheme = (name: ThemeName) => {
    setThemeSignal(name);
    try { localStorage.setItem("kaubo-theme", name); } catch { /* noop */ }
  };

  const toggleExamples = () => setExamplesExpanded((prev) => !prev);

  const setTabSize = (size: number) => {
    setTabSizeSignal(size);
    try { localStorage.setItem("kaubo-tabsize", String(size)); } catch { /* noop */ }
  };

  const setFontSize = (size: number) => {
    setFontSizeSignal(size);
    try { localStorage.setItem("kaubo-fontsize", String(size)); } catch { /* noop */ }
  };

  const toggleSettings = () => setSettingsOpen((prev) => !prev);

  const resetSettings = () => {
    setThemeSignal("material-dark");
    setTabSizeSignal(4);
    setFontSizeSignal(14);
    try {
      localStorage.setItem("kaubo-theme", "material-dark");
      localStorage.setItem("kaubo-tabsize", "4");
      localStorage.setItem("kaubo-fontsize", "14");
    } catch { /* noop */ }
  };

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

  function requireWasmResult<T>(value: T | null | undefined): T {
    if (value == null) {
      throw new Error(WASM_NOT_LOADED_MESSAGE);
    }
    return value;
  }

  const compile = () => {
    batch(() => {
      setStatus("compiling");
      setOutput("");
      setError(null);
    });
    try {
      const len = requireWasmResult(doCompile(code()));
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
        requireWasmResult(doCompile(code()));
        const out = requireWasmResult(doRun());
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

  const clearOutput = () => setOutput("");

  return {
    code, setCode: updateCode, output, status, error,
    theme, setTheme,
    tabSize, setTabSize,
    fontSize, setFontSize,
    settingsOpen, toggleSettings, resetSettings,
    activeExample, examplesExpanded, toggleExamples, loadExample,
    compile, run, clearError, clearOutput, loading,
  };
}
