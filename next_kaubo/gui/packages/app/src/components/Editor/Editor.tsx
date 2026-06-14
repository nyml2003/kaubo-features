import { onMount, createEffect, type Component } from "solid-js";
import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import styles from "./Editor.module.css";

export const Editor: Component<{
  code: () => string;
  onUpdate: (value: string) => void;
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
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              props.onUpdate(update.state.doc.toString());
            }
          }),
        ],
      }),
    });

    // Sync external code changes into the editor
    createEffect(() => {
      const external = props.code();
      if (external !== view.state.doc.toString()) {
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: external },
        });
      }
    });
  });

  return <div ref={container} class={styles.editor} />;
};
