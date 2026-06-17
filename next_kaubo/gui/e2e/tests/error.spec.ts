import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForSelector("text=Compile", { timeout: 30_000 });
});

test("empty input does not crash", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.press("Backspace");

  await page.click("button:has-text('Compile')");
  await page.waitForTimeout(2000);
  await expect(page.locator(".cm-content")).toBeVisible();
});

test("run valid code produces output", async ({ page }) => {
  const editor = page.locator(".cm-content");
  await editor.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type('print("42");');

  await page.click("button:has-text('Run')");
  await page.waitForTimeout(3000);

  // The output panel should show "42"
  const outputPanel = page.locator("pre");
  await expect(outputPanel).toContainText("42", { timeout: 5_000 });
});
