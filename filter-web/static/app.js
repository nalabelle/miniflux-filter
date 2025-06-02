/**
 * Main application logic
 * Uses vanilla JavaScript and the API functions from lib/api.js
 */

// Load stats
async function loadStats() {
  try {
    const response = await fetchStats();
    if (response.success) {
      const stats = response.data;
      document.getElementById("totalRuleSets").textContent = stats.total_rule_sets;
      document.getElementById("enabledRuleSets").textContent = stats.enabled_rule_sets;
      document.getElementById("totalRules").textContent = stats.total_rules;
    }
  } catch (error) {
    console.error("Failed to load stats:", error);
  }
}

// Extract domain from URL
function extractDomain(url) {
  try {
    const urlObj = new URL(url);
    return urlObj.hostname.replace("www.", "");
  } catch {
    return url; // Return original if URL parsing fails
  }
}

// Load and combine feeds and rule sets
async function loadCombinedData() {
  try {
    const feedsLoading = document.getElementById("feedsLoading");
    const feedsError = document.getElementById("feedsError");
    const combinedList = document.getElementById("combinedList");

    feedsLoading.style.display = "block";
    feedsError.style.display = "none";
    combinedList.style.display = "none";

    // Fetch both feeds and rule sets
    const [feedsResponse, ruleSetsResponse] = await Promise.all([fetchFeeds(), fetchRuleSets()]);

    feedsLoading.style.display = "none";

    if (!feedsResponse.success) {
      feedsError.textContent = feedsResponse.error || "Failed to load feeds";
      feedsError.style.display = "block";
      return;
    }

    const feeds = feedsResponse.data || [];
    const ruleSets = ruleSetsResponse.success ? ruleSetsResponse.data || [] : [];

    // Create a map of rule sets by feed ID for quick lookup
    const ruleSetMap = {};
    ruleSets.forEach((rs) => {
      ruleSetMap[rs.feed_id] = rs;
    });

    // Store data globally for filtering
    window.allFeeds = feeds;
    window.ruleSetMap = ruleSetMap;

    renderCombinedList();
  } catch (error) {
    console.error("Failed to load data:", error);
    document.getElementById("feedsError").textContent = "Failed to load data";
    document.getElementById("feedsError").style.display = "block";
    document.getElementById("feedsLoading").style.display = "none";
  }
}

// Render the combined list based on current filter
function renderCombinedList() {
  const combinedList = document.getElementById("combinedList");
  const filterWithRules = document.getElementById("filterWithRules").checked;

  combinedList.innerHTML = "";

  // Filter feeds based on toggle
  const feedsToShow = filterWithRules
    ? window.allFeeds.filter((feed) => window.ruleSetMap[feed.id])
    : window.allFeeds;

  feedsToShow.forEach((feed) => {
    const ruleSet = window.ruleSetMap[feed.id];
    const feedItem = document.createElement("div");
    feedItem.className = `feed-item ${ruleSet ? "has-rules" : "no-rules"}`;

    const domain = extractDomain(feed.site_url);
    const ruleInfo = ruleSet
      ? `${ruleSet.rules.length} rules | ${ruleSet.enabled ? "Enabled" : "Disabled"}`
      : "No rules";

    feedItem.innerHTML = `
      <div class="feed-info">
        <h3>${escapeHtml(feed.title)}</h3>
        <p>${domain} | ${ruleInfo}</p>
      </div>
      <div>
        ${
          ruleSet
            ? `<button class="button button-secondary" onclick="editRules(${feed.id})">Edit Rules</button>
               <button class="button button-danger" onclick="deleteRules(${feed.id})">Delete Rules</button>`
            : `<button class="button button-primary" onclick="createRules(${feed.id}, '${escapeHtml(
                feed.title
              )}')">Create Rules</button>`
        }
      </div>
    `;

    combinedList.appendChild(feedItem);
  });

  combinedList.style.display = "block";
}

// Actions
async function createRules(feedId, feedName) {
  try {
    const response = await createRuleSet({
      feed_id: feedId,
      feed_name: feedName,
    });

    if (response.success) {
      alert("Rule set created successfully!");
      loadCombinedData();
      loadStats();
    } else {
      alert("Failed to create rule set: " + response.error);
    }
  } catch (error) {
    alert("Failed to create rule set: " + error.message);
  }
}

async function deleteRules(feedId) {
  if (!confirm("Are you sure you want to delete this rule set?")) {
    return;
  }

  try {
    const response = await deleteRuleSet(feedId);

    if (response.success) {
      alert("Rule set deleted successfully!");
      loadCombinedData();
      loadStats();
    } else {
      alert("Failed to delete rule set: " + response.error);
    }
  } catch (error) {
    alert("Failed to delete rule set: " + error.message);
  }
}

function editRules(feedId) {
  window.location.href = `/static/edit.html?feed=${feedId}`;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

// Initialize when DOM is loaded
document.addEventListener("DOMContentLoaded", function () {
  loadStats();
  loadCombinedData();

  // Add event listener for filter toggle
  document.getElementById("filterWithRules").addEventListener("change", renderCombinedList);
});
