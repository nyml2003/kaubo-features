import { type Component } from "solid-js";
import { THEME_NAMES, presets, type ThemeName } from "../../themes";
import styles from "./Settings.module.css";

const FONT_SIZES = [12, 14, 16] as const;

export const Settings: Component<{
  open: boolean;
  theme: ThemeName;
  tabSize: number;
  fontSize: number;
  onThemeChange: (name: ThemeName) => void;
  onTabSizeChange: (size: number) => void;
  onFontSizeChange: (size: number) => void;
  onReset: () => void;
  onClose: () => void;
}> = (props) => {
  return (
    <>
      <div
        class={styles.backdrop}
        classList={{ [String(styles.visible)]: props.open }}
        onClick={props.onClose}
      />
      <aside
        class={styles.drawer}
        classList={{ [String(styles.visible)]: props.open }}
      >
        <div class={styles.header}>
          <h3 class={styles.heading}>Settings</h3>
          <button class={styles.closeBtn} onClick={props.onClose}>
            &times;
          </button>
        </div>

        <div class={styles.body}>
          <label class={styles.label}>
            <span class={styles.labelText}>Color Theme</span>
            <select
              class={styles.select}
              value={props.theme}
              onChange={(e) => {
                props.onThemeChange(e.currentTarget.value as ThemeName);
              }}
            >
              {THEME_NAMES.map((name) => (
                <option value={name}>{presets[name].label}</option>
              ))}
            </select>
          </label>

          <label class={styles.label}>
            <span class={styles.labelText}>Tab Size</span>
            <div class={styles.toggleRow}>
              <button
                class={styles.toggleBtn}
                classList={{
                  [String(styles.toggleActive)]: props.tabSize === 2,
                }}
                onClick={() => {
                  props.onTabSizeChange(2);
                }}
              >
                2
              </button>
              <button
                class={styles.toggleBtn}
                classList={{
                  [String(styles.toggleActive)]: props.tabSize === 4,
                }}
                onClick={() => {
                  props.onTabSizeChange(4);
                }}
              >
                4
              </button>
            </div>
          </label>

          <label class={styles.label}>
            <span class={styles.labelText}>Font Size</span>
            <div class={styles.toggleRow}>
              {FONT_SIZES.map((size) => (
                <button
                  class={styles.toggleBtn}
                  classList={{
                    [String(styles.toggleActive)]: props.fontSize === size,
                  }}
                  onClick={() => {
                    props.onFontSizeChange(size);
                  }}
                >
                  {size}px
                </button>
              ))}
            </div>
          </label>

          <div class={styles.resetRow}>
            <button class={styles.resetBtn} onClick={props.onReset}>
              Restore Defaults
            </button>
          </div>
        </div>
      </aside>
    </>
  );
};
