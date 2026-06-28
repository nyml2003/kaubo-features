import { describe, expect, it, vi } from "vitest";

vi.mock("@kaubo/wasm", () => ({
  lex: vi.fn(() => "[]"),
  semantic_tokens: vi.fn(() => "[]"),
}));

import {
  CLASS_BY_KIND,
  errorsToDiagnostics,
  mergeRanges,
  rangesOverlap,
  tokenToRange,
  tokensToRanges,
  type DecorationRange,
  type KauboError,
} from "./kauboLang";

function getAt<T>(arr: readonly T[], idx: number): T {
  const item = arr[idx];
  if (item === undefined) {
    throw new Error(`Index ${String(idx)} out of bounds`);
  }
  return item;
}

describe("CLASS_BY_KIND", () => {
  it("has all 7 token categories", () => {
    expect(Object.keys(CLASS_BY_KIND)).toHaveLength(11);
    expect(CLASS_BY_KIND.keyword).toBeTruthy();
    expect(CLASS_BY_KIND.number).toBeTruthy();
    expect(CLASS_BY_KIND.string).toBeTruthy();
    expect(CLASS_BY_KIND.comment).toBeTruthy();
    expect(CLASS_BY_KIND.identifier).toBeTruthy();
    expect(CLASS_BY_KIND.atom).toBeTruthy();
    expect(CLASS_BY_KIND.operator).toBeTruthy();
    expect(CLASS_BY_KIND.type).toBeTruthy();
    expect(CLASS_BY_KIND.field).toBeTruthy();
    expect(CLASS_BY_KIND.method).toBeTruthy();
    expect(CLASS_BY_KIND.function).toBeTruthy();
  });

  it("all values are cm-kaubo-* class names", () => {
    for (const cls of Object.values(CLASS_BY_KIND)) {
      expect(cls).toMatch(/^cm-kaubo-/);
    }
  });
});

describe("tokenToRange", () => {
  it("maps keyword token to range", () => {
    const result = tokenToRange({ kind: "keyword", from: 0, to: 3 });
    expect(result).toEqual({ from: 0, to: 3, cls: "cm-kaubo-keyword" });
  });

  it("maps number token to range", () => {
    const result = tokenToRange({ kind: "number", from: 5, to: 6 });
    expect(result).toEqual({ from: 5, to: 6, cls: "cm-kaubo-number" });
  });

  it("maps string token to range", () => {
    const result = tokenToRange({ kind: "string", from: 7, to: 14 });
    expect(result).toEqual({ from: 7, to: 14, cls: "cm-kaubo-string" });
  });

  it("maps comment token to range", () => {
    const result = tokenToRange({ kind: "comment", from: 0, to: 10 });
    expect(result).toEqual({ from: 0, to: 10, cls: "cm-kaubo-comment" });
  });

  it("maps identifier token to range", () => {
    const result = tokenToRange({ kind: "identifier", from: 4, to: 5 });
    expect(result).toEqual({ from: 4, to: 5, cls: "cm-kaubo-identifier" });
  });

  it("maps atom token to range", () => {
    const result = tokenToRange({ kind: "atom", from: 0, to: 4 });
    expect(result).toEqual({ from: 0, to: 4, cls: "cm-kaubo-atom" });
  });

  it("maps operator token to range", () => {
    const result = tokenToRange({ kind: "operator", from: 2, to: 3 });
    expect(result).toEqual({ from: 2, to: 3, cls: "cm-kaubo-operator" });
  });

  it("maps semantic tokens to ranges", () => {
    expect(tokenToRange({ kind: "type", from: 0, to: 5 })).toEqual({
      from: 0,
      to: 5,
      cls: "cm-kaubo-type",
    });
    expect(tokenToRange({ kind: "field", from: 6, to: 7 })).toEqual({
      from: 6,
      to: 7,
      cls: "cm-kaubo-field",
    });
    expect(tokenToRange({ kind: "method", from: 8, to: 11 })).toEqual({
      from: 8,
      to: 11,
      cls: "cm-kaubo-method",
    });
    expect(tokenToRange({ kind: "function", from: 12, to: 17 })).toEqual({
      from: 12,
      to: 17,
      cls: "cm-kaubo-function",
    });
  });

  it("returns null for unknown kind", () => {
    const result = tokenToRange({ kind: "garbage", from: 0, to: 1 });
    expect(result).toBeNull();
  });

  it("returns null for zero-width token", () => {
    const result = tokenToRange({ kind: "keyword", from: 5, to: 5 });
    expect(result).toBeNull();
  });

  it("returns null for reversed range", () => {
    const result = tokenToRange({ kind: "keyword", from: 5, to: 3 });
    expect(result).toBeNull();
  });
});

describe("tokensToRanges", () => {
  it("returns empty array for empty tokens", () => {
    expect(tokensToRanges([])).toEqual([]);
  });

  it("maps multiple tokens to ranges", () => {
    const tokens = [
      { kind: "keyword", from: 0, to: 3 },
      { kind: "identifier", from: 4, to: 5 },
      { kind: "operator", from: 5, to: 6 },
      { kind: "number", from: 7, to: 8 },
    ];
    const result = tokensToRanges(tokens);
    expect(result).toHaveLength(4);
    expect(result[0]).toEqual({ from: 0, to: 3, cls: "cm-kaubo-keyword" });
    expect(result[1]).toEqual({ from: 4, to: 5, cls: "cm-kaubo-identifier" });
    expect(result[2]).toEqual({ from: 5, to: 6, cls: "cm-kaubo-operator" });
    expect(result[3]).toEqual({ from: 7, to: 8, cls: "cm-kaubo-number" });
  });

  it("filters out unknown token kinds", () => {
    const tokens = [
      { kind: "keyword", from: 0, to: 3 },
      { kind: "garbage", from: 4, to: 5 },
      { kind: "number", from: 6, to: 7 },
    ];
    const result = tokensToRanges(tokens);
    expect(result).toHaveLength(2);
    expect(getAt(result, 0).cls).toBe("cm-kaubo-keyword");
    expect(getAt(result, 1).cls).toBe("cm-kaubo-number");
  });

  it("filters out zero-width tokens", () => {
    const tokens = [
      { kind: "keyword", from: 0, to: 3 },
      { kind: "number", from: 5, to: 5 },
      { kind: "operator", from: 6, to: 7 },
    ];
    const result = tokensToRanges(tokens);
    expect(result).toHaveLength(2);
  });
});

describe("errorsToDiagnostics", () => {
  it("returns empty array for empty errors", () => {
    expect(errorsToDiagnostics([])).toEqual([]);
  });

  it("converts KauboError to Diagnostic", () => {
    const errors: KauboError[] = [
      { severity: "error", from: 4, to: 5, message: "Unexpected token" },
    ];
    const result = errorsToDiagnostics(errors);
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      from: 4,
      to: 5,
      severity: "error",
      message: "Unexpected token",
    });
  });

  it("ensures to >= from + 1", () => {
    const errors: KauboError[] = [
      { severity: "error", from: 10, to: 10, message: "point error" },
    ];
    const result = errorsToDiagnostics(errors);
    expect(getAt(result, 0).to).toBe(11);
  });

  it("converts multiple errors", () => {
    const errors: KauboError[] = [
      { severity: "error", from: 0, to: 3, message: "parse error" },
      { severity: "warning", from: 5, to: 10, message: "type mismatch" },
    ];
    const result = errorsToDiagnostics(errors);
    expect(result).toHaveLength(2);
    expect(getAt(result, 0).severity).toBe("error");
    expect(getAt(result, 1).severity).toBe("warning");
  });

  it("preserves severity types", () => {
    const errors: KauboError[] = [
      { severity: "error", from: 0, to: 1, message: "e" },
      { severity: "warning", from: 1, to: 2, message: "w" },
    ];
    const result = errorsToDiagnostics(errors);
    expect(getAt(result, 0).severity).toBe("error");
    expect(getAt(result, 1).severity).toBe("warning");
  });
});

describe("rangesOverlap", () => {
  it("detects overlap when ranges intersect", () => {
    expect(
      rangesOverlap({ from: 0, to: 5, cls: "" }, { from: 3, to: 8, cls: "" }),
    ).toBe(true);
  });

  it("detects overlap when one contains the other", () => {
    expect(
      rangesOverlap({ from: 0, to: 10, cls: "" }, { from: 2, to: 5, cls: "" }),
    ).toBe(true);
  });

  it("returns false for adjacent non-overlapping", () => {
    expect(
      rangesOverlap({ from: 0, to: 3, cls: "" }, { from: 3, to: 6, cls: "" }),
    ).toBe(false);
  });

  it("returns false for disconnected ranges", () => {
    expect(
      rangesOverlap({ from: 0, to: 2, cls: "" }, { from: 5, to: 7, cls: "" }),
    ).toBe(false);
  });
});

describe("mergeRanges", () => {
  const a: DecorationRange = { from: 0, to: 3, cls: "cm-keyword" };
  const b: DecorationRange = { from: 4, to: 6, cls: "cm-number" };
  const c: DecorationRange = { from: 7, to: 10, cls: "cm-string" };
  const bUpdated: DecorationRange = { from: 4, to: 6, cls: "cm-atom" };

  it("keeps all existing when updated is empty", () => {
    const result = mergeRanges([a, b, c], []);
    expect(result).toHaveLength(3);
    expect(result).toContainEqual(a);
    expect(result).toContainEqual(b);
    expect(result).toContainEqual(c);
  });

  it("adds all updated when existing is empty", () => {
    const result = mergeRanges([], [a, b]);
    expect(result).toHaveLength(2);
  });

  it("replaces overlapping ranges", () => {
    const result = mergeRanges([a, b, c], [bUpdated]);
    expect(result).toHaveLength(3);
    expect(result).toContainEqual(a);
    expect(result).toContainEqual(bUpdated);
    expect(result).toContainEqual(c);
    expect(result).not.toContainEqual(b);
  });

  it("handles multiple updates", () => {
    const updatedA: DecorationRange = { from: 0, to: 3, cls: "cm-atom" };
    const updatedC: DecorationRange = { from: 7, to: 10, cls: "cm-comment" };
    const result = mergeRanges([a, b, c], [updatedA, updatedC]);
    expect(result).toHaveLength(3);
    expect(result).toContainEqual(updatedA);
    expect(result).toContainEqual(b);
    expect(result).toContainEqual(updatedC);
  });

  it("filters existing by dirty starts", () => {
    const result = mergeRanges(
      [a, b, c],
      [{ from: 4, to: 8, cls: "cm-keyword" }],
    );
    expect(result).toHaveLength(2);
    expect(result).toContainEqual(a);
    expect(getAt(result, 1).from).toBe(4);
    expect(getAt(result, 1).to).toBe(8);
  });
});
