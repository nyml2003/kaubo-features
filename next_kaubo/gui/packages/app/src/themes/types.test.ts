import { describe, expect, it } from "vitest";
import { THEME_NAMES } from "./types";

describe("THEME_NAMES", () => {
  it("has 5 entries", () => {
    expect(THEME_NAMES).toHaveLength(5);
  });

  it("all names are unique", () => {
    expect(new Set(THEME_NAMES).size).toBe(THEME_NAMES.length);
  });

  it("contains expected names", () => {
    expect(THEME_NAMES).toContain("material-dark");
    expect(THEME_NAMES).toContain("nord");
    expect(THEME_NAMES).toContain("gruvbox-dark");
    expect(THEME_NAMES).toContain("min-light");
    expect(THEME_NAMES).toContain("high-contrast");
  });
});
