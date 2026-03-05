// ============================================================================
// Quest Log - Consolidated Application JavaScript
// ============================================================================
// This file consolidates multiple JS modules into one for simpler loading.
// Order matters: early-execution code first, then event-driven code.
// ============================================================================

import {
  filtered as _filtered,
  mergePatch,
  root as _root,
} from "./datastar.js";

// ============================================================================
// SECTION 1: Timezone Header (runs immediately)
// ============================================================================
// Monkey-patch fetch to add X-Timezone header to all requests.
// This is required because the server needs to know the user's timezone
// to determine "today" for quest filtering. The browser knows the user's
// timezone via Intl.DateTimeFormat().resolvedOptions().timeZone,
// but there's no standard HTTP header for timezone, so we use a custom header.
(function () {
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  console.debug("[Fetch] Timezone:", timezone);
  const originalFetch = globalThis.fetch;

  globalThis.fetch = function (input, init = {}) {
    init = init || {};
    init.headers = {
      ...init.headers,
      "X-Timezone": timezone,
    };
    return originalFetch(input, init);
  };

  // On page load, check if we have a timezone cookie. If not, set it directly
  // without triggering a reload (avoids infinite reload loop with datastar).
  const cookieName = "QuestLog-TZ=";
  const cookies = document.cookie.split(";");
  let hasTimezone = false;
  for (let cookie of cookies) {
    cookie = cookie.trim();
    if (cookie.startsWith(cookieName)) {
      hasTimezone = true;
      break;
    }
  }

  // Set cookie directly if missing and timezone is valid
  if (
    !hasTimezone && timezone && timezone !== "undefined" && timezone !== null
  ) {
    const maxAge = 60 * 60 * 24 * 365; // 1 year
    document.cookie = `QuestLog-TZ=${
      encodeURIComponent(timezone)
    }; path=/; max-age=${maxAge}; SameSite=Lax`;
  }
})();

// ============================================================================
// SECTION 2: Theme Initialization (runs immediately)
// ============================================================================
// Set theme based on prefers-color-scheme
(function () {
  const DARK_QUERY = globalThis.matchMedia("(prefers-color-scheme: dark)");

  function applyTheme() {
    document.documentElement.setAttribute(
      "data-theme",
      DARK_QUERY.matches ? "dark" : "light",
    );
  }

  DARK_QUERY.addEventListener("change", applyTheme);
  applyTheme();
})();

// ============================================================================
// SECTION 3: Client ID Generation (runs immediately)
// ============================================================================
function generateClientId() {
  let clientId = sessionStorage.getItem("quest_log_client_id");
  if (!clientId) {
    clientId = crypto.randomUUID();
    sessionStorage.setItem("quest_log_client_id", clientId);
    console.debug("[Client] Generated new client ID:", clientId);
  }
  return clientId;
}

const CLIENT_ID = generateClientId();

mergePatch({ client_id: CLIENT_ID });

// ============================================================================
// SECTION 4: UI Effects - Notifications & Confetti
// ============================================================================
function pulseCounter(element) {
  element.classList.remove("exp-pulse");
  void element.offsetWidth;
  element.classList.add("exp-pulse");
  setTimeout(() => element.classList.remove("exp-pulse"), 500);
}

function appendWithViewTransition(container, notification) {
  if (!document.startViewTransition) {
    container.appendChild(notification);
    return;
  }

  const doAppend = () => {
    try {
      document.startViewTransition(() => {
        container.appendChild(notification);
      });
    } catch {
      container.appendChild(notification);
    }
  };

  const active = document.activeViewTransition;
  if (active) {
    active.finished.then(doAppend).catch(() =>
      container.appendChild(notification)
    );
  } else {
    doAppend();
  }
}

function triggerErrorNotification(message) {
  const container = document.querySelector(".notifications");
  if (!container) return;

  const notification = document.createElement("div");
  notification.className = "error-message";
  notification.textContent = message;

  appendWithViewTransition(container, notification);

  setTimeout(() => {
    notification.classList.add("notification-exit");
    setTimeout(() => notification.remove(), 300);
  }, 3000);
}

function triggerSuccessNotification(message) {
  const container = document.querySelector(".notifications");
  if (!container) return;

  const notification = document.createElement("div");
  notification.className = "success-message";
  notification.textContent = message;

  appendWithViewTransition(container, notification);

  setTimeout(() => {
    notification.classList.add("notification-exit");
    setTimeout(() => notification.remove(), 300);
  }, 3000);
}

function triggerQuestUncompletionNotification() {
  const container = document.querySelector(".notifications");
  if (!container) return;

  const notification = document.createElement("div");
  notification.className = "error-message";
  notification.textContent = "Quest reopened";

  appendWithViewTransition(container, notification);

  setTimeout(() => {
    notification.classList.add("notification-exit");
    setTimeout(() => notification.remove(), 300);
  }, 3000);
}

function triggerRewardClaimedNotification(rewardId) {
  const rewardCard = document.getElementById(`reward-${rewardId}`);
  if (rewardCard) {
    rewardCard.setAttribute("data-just-claimed", "true");
    setTimeout(() => rewardCard.removeAttribute("data-just-claimed"), 600);
  }

  const overlay = document.createElement("div");
  overlay.className = "reward-claimed-notification";
  overlay.innerHTML = `
    <h2>🎉 REWARD CLAIMED! 🎉</h2>
    <p>Your treasure awaits!</p>
  `;
  document.body.appendChild(overlay);

  createConfetti();

  setTimeout(() => {
    overlay.style.animation = "reward-notification-in 0.3s ease-out reverse";
    setTimeout(() => overlay.remove(), 300);
  }, 2500);
}

function createConfetti() {
  const colors = [
    "oklch(0.6 0.15 45)",
    "oklch(0.6 0.15 150)",
    "oklch(0.6 0.15 250)",
    "oklch(0.8 0.15 45)",
    "oklch(0.8 0.15 150)",
  ];

  for (let i = 0; i < 30; i++) {
    const particle = document.createElement("div");
    particle.className = "celebration-particle";
    particle.style.left = "50%";
    particle.style.top = "50%";
    particle.style.background =
      colors[Math.floor(Math.random() * colors.length)];
    particle.style.setProperty(
      "--particle-vx",
      `${(Math.random() - 0.5) * 300}px`,
    );
    particle.style.setProperty(
      "--particle-vy",
      `${(Math.random() - 0.5) * 300}px`,
    );
    particle.style.animationDelay = `${Math.random() * 0.2}s`;
    particle.style.animationDuration = `${0.8 + Math.random() * 0.4}s`;
    document.body.appendChild(particle);

    setTimeout(() => particle.remove(), 1500);
  }
}

function handleWeeklyChampionCelebration() {
  // Get the current week start from the day title or calculate it
  const dayTitle = document.querySelector(".day-title");
  if (!dayTitle) return;

  // Try to get the week start from the page - we'll use the current week
  const now = new Date();
  const day = now.getDay();
  const diff = now.getDate() - day + (day === 0 ? -6 : 1); // Adjust for Sunday
  const weekStart = new Date(now.setDate(diff));
  const weekStartStr = weekStart.toISOString().split("T")[0];

  const storageKey = `celebrated_for_week_${weekStartStr}`;

  // Check if we've already celebrated for this week
  if (localStorage.getItem(storageKey)) {
    console.debug("[Celebration] Already celebrated for week:", weekStartStr);
    return;
  }

  // Mark as celebrated for this week
  localStorage.setItem(storageKey, "true");
  console.debug(
    "[Celebration] Weekly Champion celebration triggered for week:",
    weekStartStr,
  );

  // Trigger extra confetti burst
  createConfetti();
  setTimeout(createConfetti, 300);
  setTimeout(createConfetti, 600);
}

// ============================================================================
// SECTION 5: Video Modal (runs on DOMContentLoaded)
// ============================================================================
function initVideoModal() {
  const STORAGE_KEY = "quest-log-first-time";

  const modal = document.getElementById("video-modal");
  const video = document.getElementById("intro-video");
  const closeBtn = document.querySelector(".video-close");

  if (!modal || !video) return;

  function openVideoModal() {
    modal.style.display = "flex";
    modal.style.opacity = "1";
    video.currentTime = 0;
    video.play().catch(function (err) {
      console.warn("[Video] Autoplay blocked:", err.message);
    });
  }

  function closeVideoModal() {
    modal.style.opacity = "0";
    modal.style.transition = "opacity 0.3s ease-out";
    setTimeout(function () {
      modal.style.display = "none";
      if (!video.paused) {
        video.pause();
      }
    }, 300);
    localStorage.setItem(STORAGE_KEY, "true");
  }

  globalThis.openVideoModal = openVideoModal;
  globalThis.closeVideoModal = closeVideoModal;

  closeBtn.addEventListener("click", closeVideoModal);

  modal.addEventListener("click", function (e) {
    if (e.target === modal || e.target.classList.contains("video-overlay")) {
      closeVideoModal();
    }
  });

  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape" && modal.style.display !== "none") {
      closeVideoModal();
    }
  });

  video.addEventListener("ended", closeVideoModal);

  if (!localStorage.getItem(STORAGE_KEY)) {
    openVideoModal();
  }
}

// ============================================================================
// SECTION 6: Quest UI - Keyboard Navigation & Mutation Observer
// ============================================================================
let previousExpToday = 0;

function initQuestUI() {
  // Keyboard navigation
  document.addEventListener("keydown", (e) => {
    if (e.key === "ArrowLeft") {
      const prevBtn = document.querySelector(".nav-btn.left:not(.disabled)");
      if (prevBtn) prevBtn.click();
    } else if (e.key === "ArrowRight") {
      const nextBtn = document.querySelector(".nav-btn.right:not(.disabled)");
      if (nextBtn) nextBtn.click();
    } else if (e.key === "t" || e.key === "T") {
      const todayBtn = document.querySelector(".today-btn");
      if (todayBtn) todayBtn.click();
    }
  });

  // Mutation observer for quest completion/uncompletion
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === "attributes" && mutation.attributeName) {
        const el = mutation.target;
        const attr = el.getAttribute(mutation.attributeName);

        if (
          mutation.attributeName === "data-just-completed" && attr === "true"
        ) {
          triggerSuccessNotification("Quest completed!");
          el.removeAttribute("data-just-completed");
        } else if (
          mutation.attributeName === "data-just-uncompleted" &&
          attr === "true"
        ) {
          triggerQuestUncompletionNotification();
          el.removeAttribute("data-just-uncompleted");
        }
      }
    }
  });

  const questList = document.getElementById("quest-list");
  if (questList) {
    observer.observe(questList, {
      attributes: true,
      subtree: true,
      attributeFilter: ["data-just-completed", "data-just-uncompleted"],
    });
  }

  // Listen to Datastar signal patches
  document.addEventListener("datastar-signal-patch", (e) => {
    const signals = e.detail;
    console.debug("[Signals] Received:", Object.keys(signals).join(", "));
    if (signals.error) {
      triggerErrorNotification(signals.error);
    }

    if (typeof signals.expToday === "number") {
      const expCounter = document.getElementById("exp-counter");
      if (expCounter && signals.expToday !== previousExpToday) {
        const diff = signals.expToday - previousExpToday;
        if (diff > 0) {
          pulseCounter(expCounter);
        }
      }
      previousExpToday = signals.expToday;
    }

    if (signals.rewardClaimed) {
      triggerRewardClaimedNotification(signals.rewardClaimed);
    }

    // Handle celebration when all rewards are claimed
    if (signals.allRewardsClaimed === true) {
      handleWeeklyChampionCelebration();
    }
  });

  // Handle browser back/forward
  addEventListener("popstate", () => {
    console.debug("[History] Browser back/forward pressed, reloading");
    location.reload();
  });
}

// ============================================================================
// SECTION 7: SSE Events - Connection & Death Screen
// ============================================================================
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
    // Prevent duplicate connections - if already connecting or connected, don't create another
    if (
      eventSource &&
      (eventSource.readyState === EventSource.CONNECTING ||
        eventSource.readyState === EventSource.OPEN)
    ) {
      console.log(
        "[SSE] Already connected or connecting, skipping duplicate connect()",
      );
      return;
    }

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

    eventSource.onerror = function _err() {
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
    // Initialize video modal and quest UI
    initVideoModal();
    initQuestUI();
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

  // Global error handler for uncaught JS errors
  globalThis.addEventListener("error", function (event) {
    console.error(
      "[JS] Uncaught error:",
      event.message,
      "at",
      event.filename,
      ":",
      event.lineno,
    );
  });

  globalThis.addEventListener("unhandledrejection", function (event) {
    console.error("[JS] Unhandled promise rejection:", event.reason);
  });
})();
