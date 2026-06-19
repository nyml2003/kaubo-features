import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForSelector("text=Compile", { timeout: 30_000 });
});

test("lex returns tokens for var x = 1;", async ({ page }) => {
  const count = await page.evaluate(() => {
    const { lex } = (window as any).__kauboWasm;
    const json = lex("var x = 1;");
    return JSON.parse(json).length;
  });
  expect(count).toBeGreaterThanOrEqual(4); // var, x, =, 1
});

test("lex returns keyword token for var", async ({ page }) => {
  const first = await page.evaluate(() => {
    const { lex } = (window as any).__kauboWasm;
    const json = lex("var x = 1;");
    return JSON.parse(json)[0];
  });
  expect(first).toBeDefined();
  expect(first.kind).toBe("keyword");
  expect(first.from).toBe(0);
  expect(first.to).toBe(3);
});

test("lex returns number token for literal", async ({ page }) => {
  const tokens = await page.evaluate(() => {
    const { lex } = (window as any).__kauboWasm;
    return JSON.parse(lex("42"));
  });
  expect(tokens.length).toBeGreaterThanOrEqual(1);
  expect(tokens[0].kind).toBe("number");
});

test("lex handles empty source", async ({ page }) => {
  const tokens = await page.evaluate(() => {
    const { lex } = (window as any).__kauboWasm;
    return JSON.parse(lex(""));
  });
  expect(tokens).toEqual([]);
});

test("semantic tokens classify type field method and function", async ({ page }) => {
  const roles = await page.evaluate(() => {
    const { semantic_tokens } = (window as any).__kauboWasm;
    const source = "struct Point { x: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1 };\np.dis();\nprint(p.x);";
    return JSON.parse(semantic_tokens(source)).map((t: { kind: string }) => t.kind);
  });
  expect(roles).toContain("type");
  expect(roles).toContain("field");
  expect(roles).toContain("method");
  expect(roles).toContain("function");
});

test("completion returns struct fields and methods after dot", async ({ page }) => {
  const labels = await page.evaluate(() => {
    const { complete } = (window as any).__kauboWasm;
    const source = "struct Point { x: Int64, y: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1, y: 2 };\np.";
    return JSON.parse(complete(source, source.length)).map((item: { label: string }) => item.label);
  });
  expect(labels).toContain("x");
  expect(labels).toContain("y");
  expect(labels).toContain("dis");
});
