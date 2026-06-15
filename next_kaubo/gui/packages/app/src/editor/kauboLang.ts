import { EditorView, Decoration, type DecorationSet } from "@codemirror/view";
import { StateField, type EditorState } from "@codemirror/state";
import { linter, type Diagnostic } from "@codemirror/lint";
import { lex } from "@kaubo/wasm";
import { log } from "../lib/logger";

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
  updated: DecorationRange[]
): DecorationRange[] {
  const dirtyStarts = new Set(updated.map((r) => r.from));
  const result = existing.filter(
    (r) =>
      !updated.some((u) => rangesOverlap(r, u)) && !dirtyStarts.has(r.from)
  );
  result.push(...updated);
  return result;
}

// ── StateField-based syntax highlighting ────────────────────────────────────

interface HighlightCache {
  source: string;
  decorations: DecorationSet;
}

const highlightCache: HighlightCache = { source: "", decorations: Decoration.none };

function buildDecorationSet(ranges: DecorationRange[]): DecorationSet {
  if (ranges.length === 0) return Decoration.none;
  const marks = ranges.map((r) =>
    Decoration.mark({ class: r.cls }).range(r.from, r.to)
  );
  return Decoration.set(marks, true);
}

function tokenize(source: string): DecorationSet {
  try {
    log.lex("deco", source.length);
    const raw = lex(source);
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

function isKauboErrorArray(items: Diagnostic[] | KauboError[]): items is KauboError[] {
  const first = items[0];
  if (first === undefined) return false;
  return "message" in first && !("severity" in first);
}

export function setKauboDiagnostics(
  diagnostics: Diagnostic[] | KauboError[] | null
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

  const kauboLinter = linter(() => kauboLintDiagnostics());

  return [highlightField, kauboLinter];
}
