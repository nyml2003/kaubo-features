import { For, type Component } from "solid-js";
import { examples, type KauboExample } from "../../examples";
import styles from "./Examples.module.css";

export const Examples: Component<{
  activeId: string | null;
  expanded: boolean;
  onSelect: (example: KauboExample) => void;
}> = (props) => {
  return (
    <aside
      class={styles.panel}
      classList={{ [String(styles.collapsed)]: !props.expanded }}
    >
      <h3 class={styles.heading}>Examples</h3>
      <ul class={styles.list}>
        <For each={examples}>
          {(ex) => (
            <li
              class={styles.item}
              classList={{ [String(styles.active)]: props.activeId === ex.id }}
              onClick={() => {
                props.onSelect(ex);
              }}
            >
              <span class={styles.name}>{ex.name}</span>
              <span class={styles.description}>{ex.description}</span>
            </li>
          )}
        </For>
      </ul>
    </aside>
  );
};
