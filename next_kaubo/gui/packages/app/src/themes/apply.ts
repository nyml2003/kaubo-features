import type { KauboTheme } from "./types";

const TOKEN_CSS_VARS: Record<keyof KauboTheme["tokens"], string> = {
  keyword: "--kb-keyword",
  number: "--kb-number",
  string: "--kb-string",
  comment: "--kb-comment",
  identifier: "--kb-identifier",
  atom: "--kb-atom",
  operator: "--kb-operator",
  type: "--kb-type",
  field: "--kb-field",
  method: "--kb-method",
  function: "--kb-function",
};

const CHROME_CSS_VARS: Record<"background" | "gutter" | "activeLine" | "selection" | "cursor" | "text", string> = {
  background: "--kb-bg",
  gutter: "--kb-gutter",
  activeLine: "--kb-active-line",
  selection: "--kb-selection",
  cursor: "--kb-cursor",
  text: "--kb-text",
};

export function applyTheme(element: HTMLElement, theme: KauboTheme): void {
  for (const [key, cssVar] of Object.entries(CHROME_CSS_VARS)) {
    element.style.setProperty(cssVar, theme[key as keyof typeof CHROME_CSS_VARS]);
  }
  for (const [tokenKind, cssVar] of Object.entries(TOKEN_CSS_VARS)) {
    element.style.setProperty(cssVar, theme.tokens[tokenKind as keyof KauboTheme["tokens"]]);
  }
}
