import type { AppStatus } from "@kaubo/types";
import { Show, type Component } from "solid-js";
import styles from "./Toolbar.module.css";

const STATUS_LABEL: Record<AppStatus, string> = {
  idle: "Ready",
  compiling: "Compiling...",
  ready: "Compiled",
  running: "Running...",
};

export const Toolbar: Component<{
  status: () => AppStatus;
  examplesExpanded: () => boolean;
  onCompile: () => void;
  onRun: () => void;
  onToggleExamples: () => void;
  onOpenSettings: () => void;
}> = (props) => {
  const busy = () =>
    props.status() === "compiling" || props.status() === "running";

  return (
    <header class={styles.toolbar}>
      <button
        class={styles.iconBtn}
        onClick={props.onToggleExamples}
        title="Toggle examples"
      >
        &#9776;
      </button>
      <span class={styles.brand}>Kaubo</span>
      <nav class={styles.actions}>
        <button class={styles.btn} disabled={busy()} onClick={props.onCompile}>
          Compile
        </button>
        <button class={styles.btn} disabled={busy()} onClick={props.onRun}>
          Run
        </button>
      </nav>
      <span class={styles.status}>
        <Show
          when={busy()}
          fallback={
            <span class={styles.ready}>{STATUS_LABEL[props.status()]}</span>
          }
        >
          <span class={styles.spin} />
          {STATUS_LABEL[props.status()]}
        </Show>
      </span>
      <button
        class={styles.iconBtn}
        onClick={props.onOpenSettings}
        title="Settings"
      >
        &#9881;
      </button>
    </header>
  );
};
