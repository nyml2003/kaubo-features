import { onMount, createEffect, type Component } from "solid-js";
import { EditorView, placeholder, keymap, highlightActiveLine, drawSelection } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { bracketMatching, foldGutter, indentOnInput, foldKeymap } from "@codemirror/language";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { lintGutter, lintKeymap } from "@codemirror/lint";
import { defaultKeymap } from "@codemirror/commands";
import { kauboLanguage } from "../../editor/kauboLang";
import { lex } from "@kaubo/wasm";
import { applyTheme, presets } from "../../themes";
import type { ThemeName } from "../../themes";
import styles from "./Editor.module.css";

declare global {
  interface Window {
    __kauboWasm?: { lex: typeof lex };
  }
}

if (typeof window !== "undefined") {
  window.__kauboWasm = { lex };
}

export const Editor: Component<{
  code: () => string;
  theme: () => string;
  onUpdate: (value: string) => void;
  onRun: () => void;
}> = (props) => {
  let container!: HTMLDivElement;
  let view: EditorView;

  onMount(() => {
    view = new EditorView({
      parent: container,
      state: EditorState.create({
        doc: props.code(),
        extensions: [
          EditorState.tabSize.of(4),
          placeholder("// Enter Kaubo code..."),
          highlightActiveLine(),
          drawSelection(),
          bracketMatching(),
          foldGutter(),
          indentOnInput(),
          closeBrackets(),
          lintGutter(),
          ...kauboLanguage(),
          keymap.of([
            ...closeBracketsKeymap,
            ...defaultKeymap,
            ...foldKeymap,
            ...lintKeymap,
            {
              key: "Ctrl-Enter",
              run: () => { props.onRun(); return true; },
            },
          ]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              props.onUpdate(update.state.doc.toString());
            }
          }),
        ],
      }),
    });

    createEffect(() => {
      const external = props.code();
      if (external !== view.state.doc.toString()) {
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: external },
        });
      }
    });

    createEffect(() => {
      const themeName = props.theme();
      const theme = presets[themeName as ThemeName];
      applyTheme(container, theme);
    });
  });

  return <div ref={container} class={styles.editor} />;
};
