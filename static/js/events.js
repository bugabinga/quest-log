import {
  filtered as _filtered,
  mergePatch,
  root as _root,
} from "./datastar.js";

function generateClientId() {
  let clientId = sessionStorage.getItem("quest_log_client_id");
  if (!clientId) {
    clientId = crypto.randomUUID();
    sessionStorage.setItem("quest_log_client_id", clientId);
  }
  return clientId;
}

const CLIENT_ID = generateClientId();

mergePatch({ client_id: CLIENT_ID });

function _parseSseData(data) {
  const result = {};
  const lines = data.split(/\n/);
  for (const line of lines) {
    const colonIdx = line.indexOf(" ");
    if (colonIdx === -1) continue;
    const key = line.slice(0, colonIdx);
    const value = line.slice(colonIdx + 1);
    (result[key] ||= []).push(value);
  }
  return Object.fromEntries(
    Object.entries(result).map(([k, v]) => [k, v.join("\n")]),
  );
}

(function () {
  "use strict";

  let eventSource = null;
  let shutdownOverlay = null;
  let isConnected = false;
  let healthCheckTimer = null;

  function startServerPolling() {
    console.log("[Shutdown] Starting server polling...");
    stopProactiveHealthCheck(); // Stop the DOMContentLoaded health check
    let pollingActive = true;
    function poll() {
      if (!pollingActive) return;
      console.log("[Shutdown] Polling /health...");
      fetch("/health", { method: "HEAD", cache: "no-cache" })
        .then(function (response) {
          console.log("[Shutdown] /health returned:", response.status);
          if (response.ok) {
            console.log("[Shutdown] Server is back! Reconnecting...");
            pollingActive = false;
            // Hide overlay immediately - don't wait for EventSource onopen which may not fire
            hideShutdownOverlay();
            connect();
          } else {
            console.log("[Shutdown] Server not ready, retrying in 2s...");
            setTimeout(poll, 2000);
          }
        })
        .catch(function (err) {
          console.log("[Shutdown] Server unreachable, retrying in 2s...", err);
          setTimeout(poll, 2000);
        });
    }
    poll();
  }

  function applyPatchElements(html) {
    const temp = document.createElement("div");
    temp.innerHTML = html;

    for (const newEl of temp.children) {
      if (!newEl.id) continue;

      const existingEl = document.getElementById(newEl.id);
      if (existingEl) {
        const newContent = newEl.innerHTML;
        const newAttrs = newEl.attributes;

        for (const attr of [...existingEl.attributes]) {
          existingEl.removeAttribute(attr.name);
        }
        for (const attr of newAttrs) {
          existingEl.setAttribute(attr.name, attr.value);
        }
        existingEl.innerHTML = newContent;

        if (
          existingEl.classList.contains("quest-item") &&
          existingEl.classList.contains("completed")
        ) {
          existingEl.setAttribute("data-just-completed", "true");
        }
      }
    }

    globalThis.datastar?.initialize?.();
  }

  function applyPatchSignals(jsonStr) {
    try {
      const signals = JSON.parse(jsonStr);
      const paths = [];
      const addPaths = (obj, prefix = "") => {
        for (const [key, value] of Object.entries(obj)) {
          const path = prefix ? `${prefix}.${key}` : key;
          if (value && typeof value === "object" && !Array.isArray(value)) {
            addPaths(value, path);
          } else {
            paths.push([path, value]);
          }
        }
      };
      addPaths(signals);

      const merged = {};
      for (const [path, value] of paths) {
        const keys = path.split(".");
        let obj = merged;
        for (let i = 0; i < keys.length - 1; i++) {
          obj[keys[i]] ||= {};
          obj = obj[keys[i]];
        }
        obj[keys[keys.length - 1]] = value;
      }

      mergePatch(merged);
    } catch (e) {
      console.error("[SSE] Error parsing signals:", e);
    }
  }

  function showShutdownOverlay() {
    if (shutdownOverlay) return;
    console.log("[SSE] 🎭 Creating shutdown overlay...");
    const overlay = document.createElement("div");
    overlay.id = "shutdown-overlay";
    overlay.innerHTML =
      '<div class="realm-shutdown"><div class="realm-content"><h1>⚔️ THE REALM REBIRTHS ⚔️</h1><span class="skull-icon">💀</span><p>The Quest Log realm is undergoing mystical regeneration...</p><p class="sub-message">Thy progress is safe. Return shortly, brave adventurer.</p><div class="progress-bar"><div class="progress-bar-fill"></div></div><div class="retry-dots"><div class="retry-dot"></div><div class="retry-dot"></div><div class="retry-dot"></div></div><p class="tip">Waiting for realm to revive...</p></div></div>';
    document.body.appendChild(overlay);
    shutdownOverlay = overlay;
    console.log("[SSE] 🎭 Overlay created and added to DOM");
  }

  function hideShutdownOverlay() {
    if (!shutdownOverlay) return;
    console.log("[SSE] 🎭 hideShutdownOverlay called");
    shutdownOverlay.remove();
    shutdownOverlay = null;
    console.log("[SSE] 🎭 Overlay removed from DOM");
  }

  // Proactive health check - detects server death even if EventSource doesn't fire onerror
  function startProactiveHealthCheck() {
    if (healthCheckTimer) return;

    function check() {
      fetch("/health", { method: "HEAD", cache: "no-cache" })
        .then(function (response) {
          if (!response.ok) {
            console.log(
              "[Health] Server returned non-OK status, showing overlay",
            );
            showShutdownOverlay();
          }
        })
        .catch(function () {
          console.log("[Health] Server unreachable, showing overlay");
          showShutdownOverlay();
        });
    }

    // Check every 5 seconds
    healthCheckTimer = setInterval(check, 5000);
    check(); // Also check immediately
  }

  function stopProactiveHealthCheck() {
    if (healthCheckTimer) {
      clearInterval(healthCheckTimer);
      healthCheckTimer = null;
    }
  }

  function connect() {
    if (eventSource) {
      eventSource.close();
    }

    console.log("[SSE] Connecting to /events...");
    eventSource = new EventSource("/events");

    eventSource.addEventListener("datastar-patch-elements", function (e) {
      console.log("[SSE] Received datastar-patch-elements");
      try {
        const payload = JSON.parse(e.data);
        if (payload.origin === CLIENT_ID) {
          console.log("[SSE] Ignoring own broadcast");
          return;
        }
        applyPatchElements(payload.data);
      } catch (err) {
        console.error("[SSE] Error applying elements:", err);
      }
    });

    eventSource.addEventListener("datastar-patch-signals", function (e) {
      console.log("[SSE] Received datastar-patch-signals");
      try {
        const payload = JSON.parse(e.data);
        if (payload.origin === CLIENT_ID) {
          console.log("[SSE] Ignoring own broadcast");
          return;
        }
        applyPatchSignals(payload.data);
      } catch (err) {
        console.error("[SSE] Error applying signals:", err);
      }
    });

    eventSource.addEventListener("server-death", function (_e) {
      // Ignore if we're connected - this is a late event from old connection
      if (isConnected) {
        console.log(
          "[SSE] Ignoring late server-death event (already connected)",
        );
        return;
      }
      console.log("[SSE] ⚠️ Server shutting down");
      showShutdownOverlay();
    });

    eventSource.addEventListener("shutdown-complete", function (_e) {
      console.log("[SSE] 🛑 Shutdown complete");
      eventSource.close();
      stopProactiveHealthCheck();
      startServerPolling();
    });

    eventSource.onopen = function () {
      isConnected = true;
      console.log("[SSE] Connected to /events");
      console.log("[SSE] 🎭 Calling hideShutdownOverlay...");
      hideShutdownOverlay();
      stopProactiveHealthCheck();
      console.log(
        "[SSE] 🎭 hideShutdownOverlay done, shutdownOverlay =",
        shutdownOverlay,
      );
    };

    eventSource.onerror = function (_err) {
      console.error("[SSE] Error");
      isConnected = false;
      eventSource.close();
      showShutdownOverlay();
      // Always use health polling - EventSource reconnection is unreliable
      console.log(
        "[SSE] Starting health polling to detect when server is back...",
      );
      startServerPolling();
    };
  }

  document.addEventListener("DOMContentLoaded", function () {
    connect();
    // Start proactive health check to detect server death even if EventSource doesn't fire onerror
    startProactiveHealthCheck();
  });

  document.addEventListener("visibilitychange", function () {
    if (
      document.visibilityState === "visible" &&
      (!eventSource || eventSource.readyState === EventSource.CLOSED)
    ) {
      console.log("[SSE] Page visible, reconnecting...");
      connect();
    }
  });
})();
