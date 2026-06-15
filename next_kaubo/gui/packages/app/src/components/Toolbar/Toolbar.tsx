import { Show, type Component } from "solid-js";
import type { AppStatus } from "@kaubo/types";
import type { ThemeName } from "../../themes";
import { THEME_NAMES, presets } from "../../themes";
import styles from "./Toolbar.module.css";

const STATUS_LABEL: Record<AppStatus, string> = {
  idle: "Ready",
  compiling: "Compiling...",
  ready: "Compiled",
  running: "Running...",
};

export const Toolbar: Component<{
  status: () => AppStatus;
  theme: () => ThemeName;
  examplesExpanded: () => boolean;
  onCompile: () => void;
  onRun: () => void;
  onThemeChange: (name: ThemeName) => void;
  onToggleExamples: () => void;
}> = (props) => {
  const busy = () => props.status() === "compiling" || props.status() === "running";

  return (
    <header class={styles.toolbar}>
      <button
        class={styles.toggleBtn}
        onClick={props.onToggleExamples}
        title="Toggle examples panel"
      >
        &#9776;
      </button>
      <span class={styles.brand}>Kaubo</span>
      <nav class={styles.actions}>
        <button
          class={styles.btn}
          disabled={busy()}
          onClick={props.onCompile}
        >
          Compile
        </button>
        <button
          class={styles.btn}
          disabled={busy()}
          onClick={props.onRun}
        >
          Run
        </button>
      </nav>
      <select
        class={styles.themeSelect}
        value={props.theme()}
        onChange={(e) => { props.onThemeChange(e.currentTarget.value as ThemeName); }}
        title="Color theme"
      >
        {THEME_NAMES.map((name) => (
          <option value={name}>{presets[name].label}</option>
        ))}
      </select>
      <span class={styles.status}>
        <Show when={busy()} fallback={
          <span class={styles.ready}>{STATUS_LABEL[props.status()]}</span>
        }>
          <span class={styles.spin} />
          {STATUS_LABEL[props.status()]}
        </Show>
      </span>
    </header>
  );
};
