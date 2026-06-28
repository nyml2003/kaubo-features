import {
  CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { describe, expect, it, vi } from "vitest";
import { kauboCompletions } from "./kauboAutocomplete";

// Mock WASM complete: returns different completions based on context
vi.mock("@kaubo/wasm", () => ({
  complete: vi.fn((source: string, offset: number) => {
    const prefix = source.slice(0, offset);

    // Dot-access: struct fields and methods
    if (prefix.endsWith("p.")) {
      return JSON.stringify([
        { label: "x", kind: "field", detail: "Point" },
        { label: "dis", kind: "method", detail: "Point method" },
      ]);
    }

    // Generic completions from WASM
    return JSON.stringify([
      { label: "var", kind: "keyword" },
      { label: "while", kind: "keyword" },
      { label: "struct", kind: "keyword" },
      { label: "pass", kind: "keyword" },
      { label: "print", kind: "function", detail: "builtin" },
      { label: "sqrt", kind: "function", detail: "builtin" },
      { label: "len", kind: "function", detail: "builtin" },
      { label: "starts_with", kind: "function", detail: "builtin" },
      { label: "true", kind: "constant" },
      { label: "PI", kind: "constant" },
    ]);
  }),
}));

function makeContext(word: string, explicit?: boolean): CompletionContext {
  const doc = `var ${word}`;
  const pos = doc.length;
  return new CompletionContext(
    EditorState.create({ doc }),
    pos,
    explicit ?? false,
  );
}

function makeRawContext(doc: string, explicit?: boolean): CompletionContext {
  return new CompletionContext(
    EditorState.create({ doc }),
    doc.length,
    explicit ?? false,
  );
}

function requireResult(ctx: CompletionContext): CompletionResult {
  const result = kauboCompletions(ctx);
  if (result === null) {
    throw new Error("Expected non-null completion result");
  }
  return result;
}

function labels(result: CompletionResult): string[] {
  return result.options.map((c) => c.label);
}

function findOption(result: CompletionResult, label: string) {
  const opt = result.options.find((c) => c.label === label);
  if (opt === undefined) {
    throw new Error(`Option "${label}" not found`);
  }
  return opt;
}

describe("kauboCompletions", () => {
  it("returns null for empty prefix when not explicit", () => {
    const ctx = makeContext("", false);
    expect(kauboCompletions(ctx)).toBeNull();
  });

  it("returns completions for explicit empty (via WASM)", () => {
    const ctx = makeContext("", true);
    expect(kauboCompletions(ctx)).not.toBeNull();
  });

  it("completes keyword 'var' via WASM", () => {
    const result = requireResult(makeContext("va"));
    expect(labels(result)).toContain("var");
  });

  it("completes 'wh' to 'while' via WASM", () => {
    const result = requireResult(makeContext("wh"));
    expect(labels(result)).toContain("while");
  });

  it("completes builtin 'print' via WASM", () => {
    const result = requireResult(makeContext("pr"));
    expect(labels(result)).toContain("print");
    expect(findOption(result, "print").type).toBe("function");
  });

  it("completes atom 'true' via WASM", () => {
    const result = requireResult(makeContext("tru"));
    expect(labels(result)).toContain("true");
    expect(findOption(result, "true").type).toBe("constant");
  });

  it("completes struct fields and methods after dot via WASM", () => {
    const result = requireResult(makeRawContext("p."));
    expect(labels(result)).toContain("x");
    expect(labels(result)).toContain("dis()");
    expect(findOption(result, "x").type).toBe("property");
    expect(findOption(result, "dis()").type).toBe("method");
  });

  it("WASM completions have boost 3", () => {
    const result = requireResult(makeContext("var"));
    const kw = findOption(result, "var");
    expect(kw.boost).toBe(3);
  });

  it("builtin completions have detail from WASM", () => {
    const result = requireResult(makeContext("len"));
    const fn = findOption(result, "len");
    expect(fn.detail).toBe("builtin");
  });
});
