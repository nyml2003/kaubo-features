import { describe, it, expect } from "vitest";
import { examples } from "./examples";

describe("examples", () => {
  it("has 10 examples", () => {
    expect(examples).toHaveLength(10);
  });

  it("all examples have unique IDs", () => {
    const ids = examples.map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  for (const ex of examples) {
    describe(`example "${ex.id}"`, () => {
      it("has a non-empty name", () => {
        expect(ex.name.length).toBeGreaterThan(0);
      });

      it("has a non-empty description", () => {
        expect(ex.description.length).toBeGreaterThan(0);
      });

      it("has non-empty code", () => {
        expect(ex.code.length).toBeGreaterThan(0);
      });

      it("has at least one tag", () => {
        expect(ex.tags.length).toBeGreaterThan(0);
      });
    });
  }
});
