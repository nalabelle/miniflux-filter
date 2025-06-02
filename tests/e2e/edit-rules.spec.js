const {test, expect} = require("@playwright/test");

test.describe("Edit Rules", () => {
  test("should navigate to edit page from dashboard", async ({page}) => {
    await page.goto("/static/");

    // Wait for feeds to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Look for a feed with rules and click Edit Rules
    const feedWithRules = page.locator(".feed-item.has-rules").first();

    if ((await feedWithRules.count()) > 0) {
      await feedWithRules.locator("button").filter({hasText: "Edit Rules"}).click();

      // Should navigate to edit page
      await expect(page).toHaveURL(/\/static\/edit\.html\?feed=\d+/);
      await expect(page).toHaveTitle(/Edit Rules/);
      await expect(page.locator("h1")).toContainText("Edit Rules");
    }
  });

  test("should handle non-existent feed correctly", async ({page}) => {
    // Go directly to edit page for a non-existent feed
    await page.goto("/static/edit.html?feed=99999");

    // Wait for loading to finish
    await expect(page.locator("#loading")).toBeHidden({timeout: 10000});

    // Should show error, not editor
    await expect(page.locator("#error")).toBeVisible();
    await expect(page.locator("#editor")).not.toBeVisible();
  });

  test("should navigate back to dashboard", async ({page}) => {
    await page.goto("/static/");
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    const anyFeed = page.locator(".feed-item").first();
    if ((await anyFeed.count()) > 0) {
      const createButton = anyFeed.locator("button").filter({hasText: "Create Rules"});
      const editButton = anyFeed.locator("button").filter({hasText: "Edit Rules"});

      if ((await createButton.count()) > 0) {
        await createButton.click();
      } else if ((await editButton.count()) > 0) {
        await editButton.click();
      }

      await expect(page.locator("#loading")).toBeHidden({timeout: 10000});

      // Click Back to Dashboard
      await page.locator("a").filter({hasText: "← Back to Dashboard"}).click();

      // Should return to dashboard
      await expect(page).toHaveURL("/static/");
      await expect(page.locator("h1")).toContainText("Miniflux Filter Manager");
    }
  });
});
