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

  // Error should appear — either in overlay header or as text
  await expect(page.getByText("ParserError")).toBeVisible({ timeout: 10_000 });
});
