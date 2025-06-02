/**
 * Pure API functions for fetching data from the server
 * No dependencies, just pure vanilla JavaScript
 */

/**
 * Fetch feeds from the API
 * @returns {Promise<{success: boolean, data?: Array, error?: string}>}
 */
async function fetchFeeds() {
  try {
    const response = await fetch("/api/feeds");
    const result = await response.json();

    if (result.success) {
      return {
        success: true,
        data: result.data || [],
      };
    } else {
      return {
        success: false,
        error: result.error || "Unknown error",
      };
    }
  } catch (error) {
    return {
      success: false,
      error: error.message,
    };
  }
}

/**
 * Fetch rule sets from the API
 * @returns {Promise<{success: boolean, data?: Array, error?: string}>}
 */
async function fetchRuleSets() {
  try {
    const response = await fetch("/api/rules");
    const result = await response.json();

    if (result.success) {
      return {
        success: true,
        data: result.data || [],
      };
    } else {
      return {
        success: false,
        error: result.error || "Unknown error",
      };
    }
  } catch (error) {
    return {
      success: false,
      error: error.message,
    };
  }
}

/**
 * Fetch stats from the API
 * @returns {Promise<{success: boolean, data?: Object, error?: string}>}
 */
async function fetchStats() {
  try {
    const response = await fetch("/api/stats");
    const result = await response.json();

    if (result.success) {
      return {
        success: true,
        data: result.data || {},
      };
    } else {
      return {
        success: false,
        error: result.error || "Unknown error",
      };
    }
  } catch (error) {
    return {
      success: false,
      error: error.message,
    };
  }
}

/**
 * Create a new rule set
 * @param {Object} ruleSetData - The rule set data
 * @returns {Promise<{success: boolean, data?: Object, error?: string}>}
 */
async function createRuleSet(ruleSetData) {
  try {
    const response = await fetch("/api/rules", {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify(ruleSetData),
    });

    const result = await response.json();
    return result;
  } catch (error) {
    return {
      success: false,
      error: error.message,
    };
  }
}

/**
 * Delete a rule set
 * @param {number} feedId - The feed ID
 * @returns {Promise<{success: boolean, data?: Object, error?: string}>}
 */
async function deleteRuleSet(feedId) {
  try {
    const response = await fetch(`/api/rules/${feedId}`, {
      method: "DELETE",
    });

    const result = await response.json();
    return result;
  } catch (error) {
    return {
      success: false,
      error: error.message,
    };
  }
}
