const {test, expect} = require("@playwright/test");

test.describe("OnClick Bug Fix", () => {
  test("should handle editRules onclick without throwing ReferenceError", async ({page}) => {
    await page.goto("/static/");

    // Listen for console errors
    const consoleErrors = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    // Listen for uncaught exceptions
    const pageErrors = [];
    page.on("pageerror", (error) => {
      pageErrors.push(error.message);
    });

    // Wait for feeds to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Find an Edit Rules button and click it
    const editButton = page.locator("button").filter({hasText: "Edit Rules"}).first();

    if ((await editButton.count()) > 0) {
      await editButton.click();

      // Wait a bit for any errors to appear
      await page.waitForTimeout(1000);

      // Check that no ReferenceError occurred
      const referenceErrors = pageErrors.filter(
        (error) => error.includes("editRules is not defined") || error.includes("ReferenceError")
      );

      expect(referenceErrors).toHaveLength(0);

      // Should successfully navigate to edit page
      await expect(page).toHaveURL(/\/static\/edit\.html\?feed=\d+/);
    }
  });

  test("should make editRules globally accessible", async ({page}) => {
    await page.goto("/static/");

    // Wait for app to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Check that editRules function exists in global scope
    const editRulesExists = await page.evaluate(() => {
      return typeof window.editRules === "function";
    });

    expect(editRulesExists).toBe(true);
  });
});
