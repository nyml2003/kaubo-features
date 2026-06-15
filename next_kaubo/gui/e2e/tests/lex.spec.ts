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
