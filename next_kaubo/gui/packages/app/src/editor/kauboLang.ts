/**
 * Kaubo syntax highlighting via WASM Lexer + CodeMirror decorations.
 */
import { EditorView, Decoration, type DecorationSet } from "@codemirror/view";
import { StateField, type EditorState } from "@codemirror/state";
import { lex } from "@kaubo/wasm";
import { log } from "../lib/logger";

interface TokenSpan {
  from: number;
  to: number;
  kind: string;
}

const CLASS_BY_KIND: Record<string, string> = {
  keyword: "cm-kaubo-keyword",
  number: "cm-kaubo-number",
  string: "cm-kaubo-string",
  comment: "cm-kaubo-comment",
  identifier: "cm-kaubo-identifier",
  atom: "cm-kaubo-atom",
  operator: "cm-kaubo-operator",
};

export function buildDecorations(source: string): DecorationSet {
  const ranges: { from: number; to: number; value: ReturnType<typeof Decoration.mark> }[] = [];

  try {
    log.lex("deco", source.length);
    const raw = lex(source);
    const tokens: TokenSpan[] = JSON.parse(raw);
    log.token("deco", tokens.length, tokens[0]);

    for (const tok of tokens) {
      const cls = CLASS_BY_KIND[tok.kind];
      if (cls && tok.from < tok.to) {
        ranges.push({
          from: tok.from,
          to: tok.to,
          value: Decoration.mark({ class: cls }),
        });
      }
    }
  } catch (e) {
    log.err("deco", e instanceof Error ? e.message : String(e));
    return Decoration.none;
  }

  if (ranges.length === 0) return Decoration.none;
  log.deco("deco", ranges.length);
  return Decoration.set(ranges, true);
}

export function kauboLanguage() {
  return StateField.define<DecorationSet>({
    create(state: EditorState): DecorationSet {
      return buildDecorations(state.doc.toString());
    },
    update(_old: DecorationSet, tr): DecorationSet {
      if (tr.docChanged) {
        return buildDecorations(tr.state.doc.toString());
      }
      return _old;
    },
    provide: (field) => EditorView.decorations.from(field),
  });
}
