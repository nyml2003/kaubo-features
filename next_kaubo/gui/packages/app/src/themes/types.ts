export interface KauboTheme {
  name: string;
  label: string;
  background: string;
  gutter: string;
  activeLine: string;
  selection: string;
  cursor: string;
  text: string;
  tokens: {
    keyword: string;
    number: string;
    string: string;
    comment: string;
    identifier: string;
    atom: string;
    operator: string;
    type: string;
    field: string;
    method: string;
    function: string;
  };
}

export type ThemeName =
  | "material-dark"
  | "nord"
  | "gruvbox-dark"
  | "min-light"
  | "high-contrast";

export const THEME_NAMES: readonly ThemeName[] = [
  "material-dark",
  "nord",
  "gruvbox-dark",
  "min-light",
  "high-contrast",
];
