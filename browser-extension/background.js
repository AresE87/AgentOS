// AgentOS Browser Extension — Background Service Worker
// Connects to the AgentOS desktop app via local API

const AGENTOS_BASE = "http://localhost:8080/api/v1";

// Context menu entries
chrome.runtime.onInstalled.addListener(() => {
  const menuItems = [
    { id: "agentos-summarize", title: "Summarize this" },
    { id: "agentos-translate", title: "Translate to Spanish" },
    { id: "agentos-explain", title: "Explain this" },
    { id: "agentos-save-memory", title: "Save to memory" },
    { id: "agentos-send", title: "Send to agent..." },
    { id: "agentos-analyze", title: "Analyze this page" },
  ];

  // Parent menu
  chrome.contextMenus.create({
    id: "agentos-parent",
    title: "AgentOS",
    contexts: ["selection", "page"],
  });

  menuItems.forEach((item) => {
    chrome.contextMenus.create({
      id: item.id,
      parentId: "agentos-parent",
      title: item.title,
      contexts: item.id === "agentos-analyze" ? ["page"] : ["selection"],
    });
  });
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const actionMap = {
    "agentos-summarize": "summarize",
    "agentos-translate": "translate",
    "agentos-explain": "explain",
    "agentos-save-memory": "save_to_memory",
    "agentos-send": "send",
    "agentos-analyze": "analyze_page",
  };

  const action = actionMap[info.menuItemId];
  if (!action) return;

  const payload = {
    action,
    selected_text: info.selectionText || "",
    tab_url: tab ? tab.url : "",
    tab_title: tab ? tab.title : "",
  };

  try {
    const token = await getStoredToken();
    const resp = await fetch(`${AGENTOS_BASE}/tasks`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(payload),
    });
    const data = await resp.json();
    console.log("AgentOS response:", data);
  } catch (err) {
    console.warn("AgentOS is not running or unreachable:", err.message);
  }
});

// Retrieve stored API token
async function getStoredToken() {
  return new Promise((resolve) => {
    chrome.storage.local.get(["agentos_token"], (result) => {
      resolve(result.agentos_token || "");
    });
  });
}

// Listen for messages from popup or content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "ping") {
    sendResponse({ status: "ok", version: "3.1.0" });
  }
  return true;
});
