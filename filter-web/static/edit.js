/**
 * Edit page functionality
 * Vanilla JavaScript for editing rules
 */

// Global variables
let currentRuleSet = null;
let currentFeedId = null;

// Initialize the page
document.addEventListener("DOMContentLoaded", function () {
  const urlParams = new URLSearchParams(window.location.search);
  const feedId = urlParams.get("feed");

  if (!feedId) {
    showError("No feed ID provided");
    return;
  }

  currentFeedId = parseInt(feedId);
  loadRuleSet(currentFeedId);
});

// API functions
async function fetchAPI(endpoint) {
  const response = await fetch(`/api${endpoint}`);
  return await response.json();
}

async function putAPI(endpoint, data) {
  const response = await fetch(`/api${endpoint}`, {
    method: "PUT",
    headers: {"Content-Type": "application/json"},
    body: JSON.stringify(data),
  });
  return await response.json();
}

// Load rule set
async function loadRuleSet(feedId) {
  try {
    const response = await fetchAPI(`/rules/${feedId}`);

    const loadingEl = document.getElementById("loading");
    const errorEl = document.getElementById("error");
    const editorEl = document.getElementById("editor");

    if (!loadingEl || !errorEl || !editorEl) {
      throw new Error("Required DOM elements not found");
    }

    loadingEl.style.display = "none";

    if (!response.success) {
      errorEl.textContent = response.error || "Failed to load rule set";
      errorEl.style.display = "block";
      return;
    }

    currentRuleSet = response.data;
    populateEditor(currentRuleSet);
    editorEl.style.display = "block";
  } catch (error) {
    console.error("Failed to load rule set:", error);
    showError("Failed to load rule set: " + error.message);
  }
}

// Show error
function showError(message) {
  const loadingEl = document.getElementById("loading");
  const errorEl = document.getElementById("error");

  if (loadingEl) loadingEl.style.display = "none";
  if (errorEl) {
    errorEl.textContent = message;
    errorEl.style.display = "block";
  }
}

// Populate editor with rule set data
function populateEditor(ruleSet) {
  const feedTitle = document.getElementById("feedTitle");
  const enabledCheckbox = document.getElementById("enabled");
  const container = document.getElementById("rulesContainer");

  if (!feedTitle || !enabledCheckbox || !container) {
    throw new Error("Required DOM elements not found");
  }

  feedTitle.textContent = `Feed ${ruleSet.feed_id}`;

  // Try to get the actual feed name from Miniflux API
  fetchAPI(`/feeds/${ruleSet.feed_id}`)
    .then((response) => {
      if (response.success && response.data) {
        feedTitle.textContent = `Feed ${ruleSet.feed_id}: ${response.data.title}`;
      }
    })
    .catch(() => {
      // Keep the fallback if API fails
    });
  enabledCheckbox.checked = ruleSet.enabled !== false;
  container.innerHTML = "";

  if (ruleSet.rules && ruleSet.rules.length > 0) {
    ruleSet.rules.forEach((rule) => {
      addRuleToContainer(rule);
    });
  } else {
    showEmptyState();
  }
}

// Show empty state
function showEmptyState() {
  const container = document.getElementById("rulesContainer");
  container.innerHTML = `
    <div class="empty-rules">
      <p>No rules defined for this feed.</p>
      <p>Click "Add Rule" to create your first filtering rule.</p>
    </div>
  `;
}

// Add rule to container
function addRuleToContainer(rule = null) {
  const container = document.getElementById("rulesContainer");

  // Remove empty state if present
  const emptyState = container.querySelector(".empty-rules");
  if (emptyState) {
    emptyState.remove();
  }

  const template = document.getElementById("ruleTemplate");
  const ruleElement = template.content.cloneNode(true);

  const ruleCard = ruleElement.querySelector(".rule-card");
  const conditionsContainer = ruleElement.querySelector(".conditions-container");

  if (rule) {
    rule.conditions.forEach((condition) => {
      addConditionToRule(conditionsContainer, condition);
    });
  } else {
    // New rule gets one empty condition
    addConditionToRule(conditionsContainer);
  }

  container.appendChild(ruleElement);
}

// Add condition to rule
function addConditionToRule(container, condition = null) {
  const template = document.getElementById("conditionTemplate");
  const conditionElement = template.content.cloneNode(true);

  if (condition) {
    const fieldSelect = conditionElement.querySelector(".condition-field");
    const operatorSelect = conditionElement.querySelector(".condition-operator");
    const valueInput = conditionElement.querySelector(".condition-value");

    fieldSelect.value = condition.field;
    operatorSelect.value = condition.operator;
    valueInput.value = condition.value;
  }

  container.appendChild(conditionElement);
}

// Event handlers
function addRule() {
  addRuleToContainer();
}

function removeRule(button) {
  const ruleCard = button.closest(".rule-card");
  ruleCard.remove();

  // Show empty state if no rules left
  const container = document.getElementById("rulesContainer");
  if (container.children.length === 0) {
    showEmptyState();
  }
}

function addCondition(button) {
  const ruleCard = button.closest(".rule-card");
  const conditionsContainer = ruleCard.querySelector(".conditions-container");
  addConditionToRule(conditionsContainer);
}

function removeCondition(button) {
  const conditionRow = button.closest(".condition-row");
  const conditionsContainer = conditionRow.parentElement;

  conditionRow.remove();

  // Ensure at least one condition remains
  if (conditionsContainer.children.length === 0) {
    addConditionToRule(conditionsContainer);
  }
}

// Save rules
async function saveRules() {
  try {
    const enabled = document.getElementById("enabled").checked;
    const rules = [];
    const validationErrors = [];

    // Collect and validate all rules
    document.querySelectorAll(".rule-card").forEach((ruleCard, ruleIndex) => {
      const conditions = [];
      ruleCard.querySelectorAll(".condition-row").forEach((conditionRow) => {
        const field = conditionRow.querySelector(".condition-field").value;
        const operator = conditionRow.querySelector(".condition-operator").value;
        const value = conditionRow.querySelector(".condition-value").value.trim();

        if (value) {
          conditions.push({field, operator, value});
        }
      });

      // Validation: check for rules without conditions
      if (conditions.length === 0) {
        validationErrors.push(`Rule ${ruleIndex + 1} has no valid conditions`);
        return;
      }

      rules.push({
        action: "markread",
        conditions,
      });
    });

    // If there are validation errors, show them and don't save
    if (validationErrors.length > 0) {
      alert("Please fix the following errors before saving:\n\n" + validationErrors.join("\n"));
      return;
    }

    // Prepare rule set
    const ruleSet = {
      feed_id: currentFeedId,
      enabled,
      rules,
    };

    // Save to server
    const response = await putAPI(`/rules/${currentFeedId}`, ruleSet);

    if (response.success) {
      alert("Rules saved successfully!");
      window.location.href = "./";
    } else {
      alert("Failed to save rules: " + response.error);
    }
  } catch (error) {
    alert("Failed to save rules: " + error.message);
  }
}

// Utility function for HTML escaping
function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
