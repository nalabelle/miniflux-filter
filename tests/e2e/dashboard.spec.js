const {test, expect} = require("@playwright/test");

test.describe("Dashboard", () => {
  test("should load dashboard and display feeds", async ({page}) => {
    // Go to the main page
    await page.goto("/static/");

    // Check that the page loads
    await expect(page).toHaveTitle(/Miniflux Filter Manager/);

    // Check that the main heading is present
    await expect(page.locator("h1")).toContainText("Miniflux Filter Manager");

    // Check that stats section is present
    await expect(page.locator(".stats")).toBeVisible();
    await expect(page.locator("#totalRuleSets")).toBeVisible();
    await expect(page.locator("#enabledRuleSets")).toBeVisible();
    await expect(page.locator("#totalRules")).toBeVisible();

    // Check that the combined section is present
    await expect(page.locator("h2").filter({hasText: "Feeds & Rules"})).toBeVisible();

    // Check that filter toggle is present
    await expect(page.locator("#filterWithRules")).toBeVisible();

    // Wait for data to load (loading should disappear)
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Check that combined list becomes visible
    await expect(page.locator("#combinedList")).toBeVisible();
  });

  test("should display combined list with filter toggle", async ({page}) => {
    await page.goto("/static/");

    // Wait for data to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Check that combined list becomes visible
    await expect(page.locator("#combinedList")).toBeVisible();

    // Check that filter toggle works
    const filterToggle = page.locator("#filterWithRules");
    await expect(filterToggle).toBeVisible();
    await expect(filterToggle).toBeChecked(); // Should default to checked

    // Toggle the filter
    await filterToggle.click();
    await expect(filterToggle).not.toBeChecked();
  });

  test("should handle feed with rules correctly", async ({page}) => {
    await page.goto("/static/");

    // Wait for feeds to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Look for a feed item that has rules
    const feedWithRules = page.locator(".feed-item.has-rules").first();

    // If there's a feed with rules, check that Edit Rules button is present
    if ((await feedWithRules.count()) > 0) {
      await expect(feedWithRules.locator("button").filter({hasText: "Edit Rules"})).toBeVisible();
      await expect(feedWithRules.locator("button").filter({hasText: "Delete Rules"})).toBeVisible();
    }
  });

  test("should handle feed without rules correctly", async ({page}) => {
    await page.goto("/static/");

    // Wait for feeds to load
    await expect(page.locator("#feedsLoading")).toBeHidden({timeout: 10000});

    // Look for a feed item that has no rules
    const feedWithoutRules = page.locator(".feed-item.no-rules").first();

    // If there's a feed without rules, check that Create Rules button is present
    if ((await feedWithoutRules.count()) > 0) {
      await expect(
        feedWithoutRules.locator("button").filter({hasText: "Create Rules"})
      ).toBeVisible();
    }
  });
});
