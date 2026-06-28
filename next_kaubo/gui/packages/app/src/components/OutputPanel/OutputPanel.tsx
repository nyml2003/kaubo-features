import { createEffect, createMemo, For, type Component } from "solid-js";
import styles from "./OutputPanel.module.css";

export const OutputPanel: Component<{
  output: () => string;
  onClear: () => void;
}> = (props) => {
  const lines = createMemo(() => props.output().split("\n"));
  let outputEl!: HTMLPreElement;

  createEffect(() => {
    const text = props.output();
    if (text) {
      outputEl.scrollTop = outputEl.scrollHeight;
    }
  });

  return (
    <div class={styles.panel}>
      <div class={styles.header}>
        <span>Output</span>
        <button
          class={styles.clearBtn}
          onClick={props.onClear}
          title="Clear output"
        >
          &times;
        </button>
      </div>
      <pre ref={outputEl} class={styles.output}>
        <For each={lines()}>{(line) => <div>{line}</div>}</For>
      </pre>
    </div>
  );
};
