import { Show, type Component } from "solid-js";
import styles from "./ErrorOverlay.module.css";

export const ErrorOverlay: Component<{
  error: () => string | null;
  onDismiss: () => void;
}> = (props) => (
  <Show when={props.error()}>
    {(msg) => (
      <div class={styles.overlay} onClick={props.onDismiss}>
        <div class={styles.box}>
          <div class={styles.header}>
            <span>Error</span>
            <button class={styles.close} onClick={props.onDismiss}>
              ×
            </button>
          </div>
          <pre class={styles.message}>{msg()}</pre>
        </div>
      </div>
    )}
  </Show>
);
