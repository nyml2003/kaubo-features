import { For, type Component } from "solid-js";
import { createMemo } from "solid-js";
import styles from "./OutputPanel.module.css";

export const OutputPanel: Component<{ output: () => string }> = (props) => {
  const lines = createMemo(() => props.output().split("\n"));

  return (
    <div class={styles.panel}>
      <div class={styles.header}>Output</div>
      <pre class={styles.output}>
        <For each={lines()}>{ (line) => <div>{line}</div> }</For>
      </pre>
    </div>
  );
};
