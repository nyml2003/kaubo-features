import { createEffect, Show, type Component } from "solid-js";
import styles from "./App.module.css";
import { Editor } from "./components/Editor/Editor";
import { ErrorOverlay } from "./components/ErrorOverlay/ErrorOverlay";
import { Examples } from "./components/Examples/Examples";
import { OutputPanel } from "./components/OutputPanel/OutputPanel";
import { Settings } from "./components/Settings/Settings";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { createKauboStore } from "./store/app";
import { applyTheme, presets } from "./themes";

export const App: Component = () => {
  const store = createKauboStore();

  createEffect(() => {
    const root = document.documentElement;
    const theme = presets[store.theme()];
    applyTheme(root, theme);
    root.style.setProperty("--kb-font-size", `${String(store.fontSize())}px`);
  });

  return (
    <div class={styles.layout}>
      <Show
        when={!store.loading()}
        fallback={<div class={styles.splash}>Loading Kaubo WASM...</div>}
      >
        <Toolbar
          status={store.status}
          examplesExpanded={store.examplesExpanded}
          onFormat={store.format}
          onCompile={store.compile}
          onRun={store.run}
          onToggleExamples={store.toggleExamples}
          onOpenSettings={store.toggleSettings}
        />
        <div class={styles.body}>
          <Examples
            activeId={store.activeExample()}
            expanded={store.examplesExpanded()}
            onSelect={store.loadExample}
          />
          <main class={styles.main}>
            <Editor
              code={store.code}
              tabSize={store.tabSize}
              onUpdate={store.setCode}
              onRun={store.run}
              onFormat={store.format}
            />
            <OutputPanel output={store.output} onClear={store.clearOutput} />
          </main>
        </div>
        <ErrorOverlay error={store.error} onDismiss={store.clearError} />
        <Settings
          open={store.settingsOpen()}
          theme={store.theme()}
          tabSize={store.tabSize()}
          fontSize={store.fontSize()}
          onThemeChange={store.setTheme}
          onTabSizeChange={store.setTabSize}
          onFontSizeChange={store.setFontSize}
          onReset={store.resetSettings}
          onClose={store.toggleSettings}
        />
      </Show>
    </div>
  );
};
