import { beforeEach, describe, expect, it } from "vitest";
import { applyTheme } from "./apply";
import { presets } from "./presets";

describe("applyTheme", () => {
  let el: HTMLElement;

  beforeEach(() => {
    el = document.createElement("div");
  });

  it("sets --kb-bg from theme background", () => {
    applyTheme(el, presets["material-dark"]);
    expect(el.style.getPropertyValue("--kb-bg")).toBe("#1a1a2e");
  });

  it("sets --kb-keyword from theme tokens", () => {
    applyTheme(el, presets["material-dark"]);
    expect(el.style.getPropertyValue("--kb-keyword")).toBe("#c792ea");
  });

  it("sets all 17 CSS custom properties", () => {
    applyTheme(el, presets["material-dark"]);
    const vars = [
      "--kb-bg",
      "--kb-gutter",
      "--kb-active-line",
      "--kb-selection",
      "--kb-cursor",
      "--kb-text",
      "--kb-keyword",
      "--kb-number",
      "--kb-string",
      "--kb-comment",
      "--kb-identifier",
      "--kb-atom",
      "--kb-operator",
      "--kb-type",
      "--kb-field",
      "--kb-method",
      "--kb-function",
    ];
    for (const v of vars) {
      expect(el.style.getPropertyValue(v)).toBeTruthy();
    }
  });

  it("switches colors when theme changes", () => {
    applyTheme(el, presets["material-dark"]);
    expect(el.style.getPropertyValue("--kb-bg")).toBe("#1a1a2e");

    applyTheme(el, presets["min-light"]);
    expect(el.style.getPropertyValue("--kb-bg")).toBe("#fafafa");
  });
});
