import { describe, expect, it } from "vitest";
import { presets } from "./presets";
import type { KauboTheme } from "./types";
import { THEME_NAMES } from "./types";

const TOKEN_KEYS: (keyof KauboTheme["tokens"])[] = [
  "keyword",
  "number",
  "string",
  "comment",
  "identifier",
  "atom",
  "operator",
  "type",
  "field",
  "method",
  "function",
];

const CHROME_KEYS: (keyof Pick<
  KauboTheme,
  "background" | "gutter" | "activeLine" | "selection" | "cursor" | "text"
>)[] = ["background", "gutter", "activeLine", "selection", "cursor", "text"];

describe("presets", () => {
  it("has a preset for every theme name", () => {
    for (const name of THEME_NAMES) {
      expect(presets[name]).toBeDefined();
    }
  });

  for (const name of THEME_NAMES) {
    describe(`preset "${name}"`, () => {
      const theme = presets[name];

      it("has a non-empty label", () => {
        expect(theme.label.length).toBeGreaterThan(0);
      });

      it("has name matching key", () => {
        expect(theme.name).toBe(name);
      });

      for (const key of CHROME_KEYS) {
        it(`defines ${key}`, () => {
          expect(theme[key]).toBeTruthy();
          expect(theme[key]).toMatch(/^#[0-9a-fA-F]{6}$/);
        });
      }

      it("has all 11 token kind colors", () => {
        expect(Object.keys(theme.tokens)).toHaveLength(11);
      });

      for (const kind of TOKEN_KEYS) {
        it(`token "${kind}" is a valid hex color`, () => {
          expect(theme.tokens[kind]).toMatch(/^#[0-9a-fA-F]{6}$/);
        });
      }
    });
  }
});
