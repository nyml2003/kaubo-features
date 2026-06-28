import { autocompletion } from "@codemirror/autocomplete";
import { linter, type Diagnostic } from "@codemirror/lint";
import { StateField, type EditorState } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  hoverTooltip,
  WidgetType,
  type DecorationSet,
  type Tooltip,
} from "@codemirror/view";
import { inlay_hints, semantic_tokens, hover as wasmHover } from "@kaubo/wasm";
import { log } from "../lib/logger";
import { kauboCompletions } from "./kauboAutocomplete";

interface TokenSpan {
  from: number;
  to: number;
  kind: string;
}

export interface KauboError {
  severity: "error" | "warning";
  from: number;
  to: number;
  message: string;
}

export const CLASS_BY_KIND: Record<string, string> = {
  keyword: "cm-kaubo-keyword",
  number: "cm-kaubo-number",
  string: "cm-kaubo-string",
  comment: "cm-kaubo-comment",
  identifier: "cm-kaubo-identifier",
  atom: "cm-kaubo-atom",
  operator: "cm-kaubo-operator",
  type: "cm-kaubo-type",
  field: "cm-kaubo-field",
  method: "cm-kaubo-method",
  function: "cm-kaubo-function",
};

export interface DecorationRange {
  from: number;
  to: number;
  cls: string;
}

/**
 * Pure function: map a token to a decoration range (testable without CodeMirror).
 */
export function tokenToRange(token: TokenSpan): DecorationRange | null {
  const cls = CLASS_BY_KIND[token.kind];
  if (!cls || token.from >= token.to) return null;
  return { from: token.from, to: token.to, cls };
}

/**
 * Pure function: map an array of tokens to decoration ranges.
 */
export function tokensToRanges(tokens: TokenSpan[]): DecorationRange[] {
  const ranges: DecorationRange[] = [];
  for (const tok of tokens) {
    const r = tokenToRange(tok);
    if (r) ranges.push(r);
  }
  return ranges;
}

/**
 * Pure function: convert Kaubo structured errors to CodeMirror Diagnostics.
 */
export function errorsToDiagnostics(errors: KauboError[]): Diagnostic[] {
  return errors.map((e) => ({
    from: e.from,
    to: Math.max(e.to, e.from + 1),
    severity: e.severity,
    message: e.message,
  }));
}

/**
 * Pure function: determine if two decoration ranges overlap.
 */
export function rangesOverlap(a: DecorationRange, b: DecorationRange): boolean {
  return a.from < b.to && b.from < a.to;
}

/**
 * Pure function: merge two arrays of decoration ranges, deduplicating overlapping
 * ranges (keeping the later one).
 */
export function mergeRanges(
  existing: DecorationRange[],
  updated: DecorationRange[],
): DecorationRange[] {
  const dirtyStarts = new Set(updated.map((r) => r.from));
  const result = existing.filter(
    (r) =>
      !updated.some((u) => rangesOverlap(r, u)) && !dirtyStarts.has(r.from),
  );
  result.push(...updated);
  return result;
}

// ── StateField-based syntax highlighting ────────────────────────────────────

interface HighlightCache {
  source: string;
  decorations: DecorationSet;
}

const highlightCache: HighlightCache = {
  source: "",
  decorations: Decoration.none,
};

function buildDecorationSet(ranges: DecorationRange[]): DecorationSet {
  if (ranges.length === 0) return Decoration.none;
  const marks = ranges.map((r) =>
    Decoration.mark({ class: r.cls }).range(r.from, r.to),
  );
  return Decoration.set(marks, true);
}

function tokenize(source: string): DecorationSet {
  try {
    log.lex("deco", source.length);
    const raw = semantic_tokens(source);
    const parsed: TokenSpan[] = JSON.parse(raw) as TokenSpan[];
    log.token("deco", parsed.length, parsed[0]);
    const ranges = tokensToRanges(parsed);
    log.deco("deco", ranges.length);
    return buildDecorationSet(ranges);
  } catch (e) {
    log.err("deco", e instanceof Error ? e.message : String(e));
    return Decoration.none;
  }
}

export function buildDecorations(source: string): DecorationSet {
  if (source === highlightCache.source) {
    return highlightCache.decorations;
  }
  const deco = tokenize(source);
  highlightCache.source = source;
  highlightCache.decorations = deco;
  return deco;
}

// ── Lint source for error diagnostics ────────────────────────────────────────

let lastDiagnostics: Diagnostic[] = [];

function isKauboErrorArray(
  items: Diagnostic[] | KauboError[],
): items is KauboError[] {
  const first = items[0];
  if (first === undefined) return false;
  return "message" in first && !("severity" in first);
}

export function setKauboDiagnostics(
  diagnostics: Diagnostic[] | KauboError[] | null,
) {
  if (diagnostics === null || diagnostics.length === 0) {
    lastDiagnostics = [];
    return;
  }
  if (isKauboErrorArray(diagnostics)) {
    lastDiagnostics = errorsToDiagnostics(diagnostics);
  } else {
    lastDiagnostics = diagnostics;
  }
}

export function kauboLintDiagnostics(): Diagnostic[] {
  return lastDiagnostics;
}

// ── Hover tooltip source ─────────────────────────────────────────────────────

interface HoverInfo {
  kind: string;
  type?: string;
  from: number;
  to: number;
  description: string;
}

function hoverSource(view: EditorView, pos: number): Tooltip | null {
  try {
    const source = view.state.doc.toString();
    const raw = wasmHover(source, pos);
    if (raw === "null") return null;
    const info: HoverInfo = JSON.parse(raw) as HoverInfo;
    return {
      pos: info.from,
      end: info.to,
      above: true,
      create() {
        const dom = document.createElement("div");
        dom.className = "cm-tooltip-hover";
        if (info.type) {
          const typeEl = document.createElement("div");
          typeEl.className = "cm-tooltip-section";
          typeEl.textContent = `${info.kind}: ${info.type}`;
          dom.append(typeEl);
        } else {
          const kind = document.createElement("div");
          kind.className = "cm-tooltip-section";
          kind.textContent = info.kind;
          dom.append(kind);
        }
        if (info.description) {
          const desc = document.createElement("div");
          desc.className = "cm-tooltip-section";
          desc.textContent = info.description;
          dom.append(desc);
        }
        return { dom };
      },
    };
  } catch {
    return null;
  }
}

// ── Inlay hints (type annotations as inline widgets) ─────────────────────────

interface InlayHint {
  position: number;
  label: string;
}

function buildInlayHints(source: string): DecorationSet {
  try {
    const raw = inlay_hints(source);
    if (raw === "[]") return Decoration.none;
    const hints: InlayHint[] = JSON.parse(raw);
    const marks = hints.map((h) =>
      Decoration.widget({
        widget: new (class extends WidgetType {
          toDOM() {
            const span = document.createElement("span");
            span.className = "cm-kaubo-inlay-hint";
            span.textContent = h.label;
            return span;
          }
        })(),
        side: 1, // after the character
      }).range(h.position),
    );
    return Decoration.set(marks, true);
  } catch {
    return Decoration.none;
  }
}

// ── Main extension ──────────────────────────────────────────────────────────

export function kauboLanguage() {
  const highlightField = StateField.define<DecorationSet>({
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

  const kauboLinter = linter((view) => {
    const diags = kauboLintDiagnostics();
    const len = view.state.doc.length;
    return diags.filter((d) => d.from < len && d.to <= len);
  });
  const kauboCompletion = autocompletion({ override: [kauboCompletions] });
  const kauboHover = hoverTooltip(hoverSource);

  const inlayHintField = StateField.define<DecorationSet>({
    create(state: EditorState): DecorationSet {
      return buildInlayHints(state.doc.toString());
    },
    update(_old: DecorationSet, tr): DecorationSet {
      if (tr.docChanged) {
        return buildInlayHints(tr.state.doc.toString());
      }
      return _old;
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  return [highlightField, kauboLinter, kauboCompletion, kauboHover, inlayHintField];
}
