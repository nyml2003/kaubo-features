import { describe, it, expect, vi } from "vitest";
import { CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { kauboCompletions } from "./kauboAutocomplete";

vi.mock("@kaubo/wasm", () => ({
  complete: vi.fn((source: string, offset: number) => {
    if (source.slice(0, offset).endsWith("p.")) {
      return JSON.stringify([
        { label: "x", kind: "field", detail: "Point" },
        { label: "dis", kind: "method", detail: "Point method" },
      ]);
    }
    return "[]";
  }),
}));

function makeContext(word: string, explicit?: boolean): CompletionContext {
  const doc = `var ${word}`;
  const pos = doc.length;
  return new CompletionContext(
    EditorState.create({ doc }),
    pos,
    explicit ?? false
  );
}

function makeRawContext(doc: string, explicit?: boolean): CompletionContext {
  return new CompletionContext(
    EditorState.create({ doc }),
    doc.length,
    explicit ?? false
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

  it("returns completions for explicit empty", () => {
    const ctx = makeContext("", true);
    expect(kauboCompletions(ctx)).not.toBeNull();
  });

  it("completes keyword prefix 'va' to 'var'", () => {
    const result = requireResult(makeContext("va"));
    expect(labels(result)).toContain("var");
  });

  it("completes 'wh' to 'while'", () => {
    const result = requireResult(makeContext("wh"));
    expect(labels(result)).toContain("while");
  });

  it("completes 'pr' to 'print'", () => {
    const result = requireResult(makeContext("pr"));
    expect(labels(result)).toContain("print");
  });

  it("completes builtin 'sq' to 'sqrt'", () => {
    const result = requireResult(makeContext("sq"));
    expect(labels(result)).toContain("sqrt");
  });

  it("completes constant 'PI'", () => {
    const result = requireResult(makeContext("PI"));
    expect(labels(result)).toContain("PI");
  });

  it("completes atom 'tru' to 'true'", () => {
    const result = requireResult(makeContext("tru"));
    expect(labels(result)).toContain("true");
  });

  it("completes 'st' to 'struct' and 'starts_with'", () => {
    const result = requireResult(makeContext("st"));
    expect(labels(result)).toContain("struct");
    expect(labels(result)).toContain("starts_with");
  });

  it("keyword completions have type keyword", () => {
    const result = requireResult(makeContext("var"));
    const kw = findOption(result, "var");
    expect(kw.type).toBe("keyword");
  });

  it("builtin completions have type function", () => {
    const result = requireResult(makeContext("len"));
    const fn = findOption(result, "len");
    expect(fn.type).toBe("function");
  });

  it("atom completions have type constant", () => {
    const result = requireResult(makeContext("true"));
    const atom = findOption(result, "true");
    expect(atom.type).toBe("constant");
  });

  it("returns null for unknown prefix", () => {
    const ctx = makeContext("xyz");
    expect(kauboCompletions(ctx)).toBeNull();
  });

  it("keywords have higher boost than builtins", () => {
    const result = requireResult(makeContext("p"));
    const passKw = findOption(result, "pass");
    const printFn = findOption(result, "print");
    if (passKw.boost === undefined || printFn.boost === undefined) {
      throw new Error("Missing boost values");
    }
    expect(passKw.boost > printFn.boost).toBe(true);
  });

  it("completes struct fields and methods after dot", () => {
    const result = requireResult(makeRawContext("p."));
    expect(labels(result)).toContain("x");
    expect(labels(result)).toContain("dis");
    expect(findOption(result, "x").type).toBe("property");
    expect(findOption(result, "dis").type).toBe("method");
  });
});
