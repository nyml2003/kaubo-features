import type { Component } from "solid-js";
import { Show } from "solid-js";
import { Editor } from "./components/Editor/Editor";
import { OutputPanel } from "./components/OutputPanel/OutputPanel";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { ErrorOverlay } from "./components/ErrorOverlay/ErrorOverlay";
import { Examples } from "./components/Examples/Examples";
import { createKauboStore } from "./store/app";
import styles from "./App.module.css";

export const App: Component = () => {
  const store = createKauboStore();

  return (
    <div class={styles.layout}>
      <Show when={!store.loading()} fallback={
        <div class={styles.splash}>Loading Kaubo WASM...</div>
      }>
        <Toolbar
          status={store.status}
          theme={store.theme}
          examplesExpanded={store.examplesExpanded}
          onCompile={store.compile}
          onRun={store.run}
          onThemeChange={store.setTheme}
          onToggleExamples={store.toggleExamples}
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
              theme={store.theme}
              onUpdate={store.setCode}
              onRun={store.run}
            />
            <OutputPanel output={store.output} />
          </main>
        </div>
        <ErrorOverlay error={store.error} onDismiss={store.clearError} />
      </Show>
    </div>
  );
};
