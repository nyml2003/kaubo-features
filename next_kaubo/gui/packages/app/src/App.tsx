import type { Component } from "solid-js";
import { Show } from "solid-js";
import { Editor } from "./components/Editor/Editor";
import { OutputPanel } from "./components/OutputPanel/OutputPanel";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { ErrorOverlay } from "./components/ErrorOverlay/ErrorOverlay";
import { createKauboStore } from "./store/app";
import styles from "./App.module.css";

export const App: Component = () => {
  const store = createKauboStore();

  return (
    <div class={styles.layout}>
      <Show when={!store.loading()} fallback={
        <div class={styles.splash}>Loading Kaubo WSM...</div>
      }>
        <Toolbar
          status={store.status}
          onCompile={store.compile}
          onRun={store.run}
        />
        <main class={styles.main}>
          <Editor code={store.code} onUpdate={store.setCode} />
          <OutputPanel output={store.output} />
        </main>
        <ErrorOverlay error={store.error} onDismiss={store.clearError} />
      </Show>
    </div>
  );
};
