import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForSelector("text=Compile", { timeout: 30_000 });
});

test("page loads with compile button", async ({ page }) => {
  await expect(page.locator("text=Compile")).toBeVisible();
  await expect(page.locator("text=Run")).toBeVisible();
});

test("compile and run hello world", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type('print("hi");');

  // Click Compile
  await page.click("button:has-text('Compile')");
  await expect(page.locator("text=Compiled:")).toBeVisible({ timeout: 10_000 });

  // Click Run — this auto-compiles, no need to compile first
  await page.click("button:has-text('Run')");

  // The output should contain something
  await page.waitForTimeout(2000);
  const outputText = await page.locator("pre").textContent();
  expect(outputText).toBeTruthy();
});

test("invalid code shows error", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type("var x = ;");

  await page.click("button:has-text('Compile')");

  // Error should appear somewhere — either output panel or diagnostics
  await page.waitForTimeout(2000);
  // Error overlay should show the error message text
  const errorMsg = page.getByText("unexpected", { exact: false });
  await expect(errorMsg.first()).toBeVisible({ timeout: 5000 });
});

test("compiled expression runs and shows output", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type('print(42.to_string());');

  await page.click("button:has-text('Run')");
  // Output should contain the printed string
  await page.waitForTimeout(2000);
  const outputText = await page.locator("pre").textContent();
  expect(outputText).toContain("42");
});

test("lambda add prints result", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type('const add = |a, b| { return a + b; }; print(add(2, 3).to_string());');

  await page.click("button:has-text('Run')");
  await page.waitForTimeout(2000);
  const outputText = await page.locator("pre").textContent();
  expect(outputText).toContain("5", { timeout: 5_000 });
});

test("struct instantiation and field access", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type('struct Point { x: Int64, y: Int64 };\nconst p = Point { x: 200, y: 100 };\nprint(p.x.to_string());');

  await page.click("button:has-text('Run')");
  await page.waitForTimeout(2000);
  const outputText = await page.locator("pre").textContent();
  expect(outputText).toContain("200", { timeout: 5_000 });
});
