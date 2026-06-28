import {
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import {
  bracketMatching,
  foldGutter,
  foldKeymap,
  indentOnInput,
} from "@codemirror/language";
import { lintGutter, lintKeymap } from "@codemirror/lint";
import { Compartment, EditorState } from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers,
  placeholder,
} from "@codemirror/view";
import { complete, goto_def, lex, semantic_tokens } from "@kaubo/wasm";
import { createEffect, onMount, type Component } from "solid-js";
import { kauboLanguage } from "../../editor/kauboLang";
import styles from "./Editor.module.css";

declare global {
  interface Window {
    __kauboWasm?: {
      lex: typeof lex;
      semantic_tokens: typeof semantic_tokens;
      complete: typeof complete;
    };
  }
}

if (typeof window !== "undefined") {
  window.__kauboWasm = { lex, semantic_tokens, complete };
}

const tabSizeComp = new Compartment();

export const Editor: Component<{
  code: () => string;
  tabSize: () => number;
  onUpdate: (value: string) => void;
  onRun: () => void;
  onFormat?: () => void;
}> = (props) => {
  let container!: HTMLDivElement;
  let view: EditorView;

  onMount(() => {
    view = new EditorView({
      parent: container,
      state: EditorState.create({
        doc: props.code(),
        extensions: [
          lineNumbers(),
          tabSizeComp.of(EditorState.tabSize.of(props.tabSize())),
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
            ...completionKeymap,
            ...foldKeymap,
            ...lintKeymap,
            {
              key: "Ctrl-Enter",
              run: () => {
                props.onRun();
                return true;
              },
            },
            {
              key: "Shift-Alt-f",
              run: () => {
                props.onFormat?.();
                return true;
              },
            },
          ]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              props.onUpdate(update.state.doc.toString());
            }
          }),
          EditorView.domEventHandlers({
            mousedown(event, view) {
              if (!event.ctrlKey && !event.metaKey) return false;
              const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
              if (pos == null) return false;
              const source = view.state.doc.toString();
              try {
                const raw = goto_def(source, pos);
                if (raw === "null") return false;
                const target: { line: number; col: number } = JSON.parse(raw);
                const line = view.state.doc.line(Math.max(1, target.line));
                const col = Math.max(0, target.col - 1);
                const targetPos = Math.min(line.from + col, line.to);
                view.dispatch({
                  selection: { anchor: targetPos },
                  scrollIntoView: true,
                });
                return true;
              } catch {
                return false;
              }
            },
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
      view.dispatch({
        effects: tabSizeComp.reconfigure(
          EditorState.tabSize.of(props.tabSize()),
        ),
      });
    });
  });

  return <div ref={container} class={styles.editor} />;
};
