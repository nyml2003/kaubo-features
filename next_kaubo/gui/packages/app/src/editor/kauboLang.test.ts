import { describe, it, expect, vi, beforeEach } from "vitest";
import { Decoration } from "@codemirror/view";

vi.mock("@kaubo/wasm", () => ({
  lex: vi.fn(() => "[]"),
}));

import { buildDecorations } from "./kauboLang";
import { lex } from "@kaubo/wasm";

function mockLex(tokens: { k: string; f: number; t: number }[]) {
  vi.mocked(lex).mockReturnValue(JSON.stringify(tokens));
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("mock plumbing", () => {
  it("lex mock returns fake data", () => {
    mockLex([{ k: "keyword", f: 0, t: 3 }]);
    const raw = lex("var");
    const parsed = JSON.parse(raw);
    expect(parsed).toEqual([{ k: "keyword", f: 0, t: 3 }]);
  });
});

describe("buildDecorations", () => {
  it("returns empty set for empty tokens", () => {
    mockLex([]);
    const decos = buildDecorations("var");
    expect(decos).toBe(Decoration.none);
  });

  // jsdom limitation: Decoration.set() returns none in vitest, works in browser.
  // Verified by e2e: compile.spec.ts, error.spec.ts test the full rendering pipeline.
  it.skip("returns non-empty set for keywords", () => {
    mockLex([
      { k: "keyword", f: 0, t: 3 },
      { k: "identifier", f: 4, t: 5 },
      { k: "operator", f: 5, t: 6 },
      { k: "number", f: 7, t: 8 },
      { k: "operator", f: 8, t: 9 },
    ]);
    const decos = buildDecorations("var x=1;");
    expect(decos).not.toBe(Decoration.none);
  });

  it("skips unknown kind", () => {
    mockLex([{ k: "garbage", f: 0, t: 1 }]);
    const decos = buildDecorations("?");
    expect(decos).toBe(Decoration.none);
  });

  it("returns empty on WASM crash", () => {
    vi.mocked(lex).mockImplementation(() => {
      throw new Error("WASM crash");
    });
    const decos = buildDecorations("x");
    expect(decos).toBe(Decoration.none);
  });

  it("returns empty on malformed JSON", () => {
    vi.mocked(lex).mockReturnValue("not json");
    const decos = buildDecorations("x");
    expect(decos).toBe(Decoration.none);
  });
});
