// ============================================================================
// Quest Log - Consolidated Application JavaScript
// ============================================================================
// This file consolidates multiple JS modules into one for simpler loading.
// Order matters: early-execution code first, then event-driven code.
// ============================================================================

// ============================================================================
// SECTION 0: Timezone Bootstrap (runs before Datastar)
// ============================================================================
const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
const timezoneCookieName = "QuestLog-TZ";

function readCookie(name) {
  const prefix = `${name}=`;
  return document.cookie
    .split(";")
    .map((cookie) => cookie.trim())
    .find((cookie) => cookie.startsWith(prefix))
    ?.slice(prefix.length);
}

if (timezone && timezone !== "undefined") {
  const cookieTimezone = readCookie(timezoneCookieName);
  if (cookieTimezone !== timezone) {
    const maxAge = 60 * 60 * 24 * 365;
    document.cookie =
      `${timezoneCookieName}=${timezone}; path=/; max-age=${maxAge}; SameSite=Lax`;
    if (readCookie(timezoneCookieName) === timezone) {
      globalThis.location.reload();
      await new Promise(() => {});
    }
  }
}

const { mergePatch } = await import("../vendor/datastar.js");

// ============================================================================
// SECTION 1: Datastar Error Interceptor
// ============================================================================
// Global error handler for Datastar - catches and logs all Datastar errors
// including GenerateExpression errors which are otherwise hard to debug.
document.addEventListener("datastar-fetch", (e) => {
  const detail = e.detail;
  if (detail?.argsRaw) {
    if (detail.argsRaw.error) {
      console.error("[Datastar]", detail.type, detail.argsRaw.error);
    } else if (detail.argsRaw.message) {
      console.error("[Datastar]", detail.type, detail.argsRaw);
    }
  }
});

// ============================================================================
// SECTION 2: Timezone Header
// ============================================================================
// Monkey-patch fetch to add X-Timezone header to all requests.
// This is required because the server needs to know the user's timezone
// to determine "today" for quest filtering. The browser knows the user's
// timezone via Intl.DateTimeFormat().resolvedOptions().timeZone,
// but there's no standard HTTP header for timezone, so we use a custom header.
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

globalThis.triggerErrorNotification = triggerErrorNotification;
globalThis.triggerSuccessNotification = triggerSuccessNotification;
globalThis.triggerQuestUncompletionNotification =
  triggerQuestUncompletionNotification;

// ============================================================================
// SECTION 5: Video Modal (runs on DOMContentLoaded)
// ============================================================================
function initVideoModal() {
  const STORAGE_KEY = "quest-log-first-time";

  const modal = document.getElementById("video-modal");
  const video = document.getElementById("intro-video");
  const closeBtn = document.querySelector(".video-close");
  const trailerBtn = document.querySelector(".nav-link--trailer");

  if (!modal || !video || !closeBtn || !trailerBtn) return;

  function openVideoModal(muted = false) {
    modal.style.display = "flex";
    modal.style.opacity = "1";
    video.currentTime = 0;
    video.muted = muted;
    video.play().catch(function (err) {
      console.warn("[Video] Playback blocked:", err.message);
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

  trailerBtn.addEventListener("click", () => openVideoModal());
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

  if (
    !localStorage.getItem(STORAGE_KEY) &&
    location.pathname !== "/editor"
  ) {
    openVideoModal(true);
  }
}

// ============================================================================
// SECTION 6: Quest UI - Keyboard Navigation & Mutation Observer
// ============================================================================
let previousExpToday = 0;

function canHandleKeyboardShortcut(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
    return false;
  }

  const target = event.target;
  if (
    target instanceof Element &&
    target.closest("input, textarea, select, [contenteditable]")
  ) {
    return false;
  }

  return ![
    ...document.querySelectorAll(
      "dialog[open], [aria-modal='true'], .auth-modal, #video-modal",
    ),
  ].some((modal) => getComputedStyle(modal).display !== "none");
}

function initQuestUI() {
  document.addEventListener("keydown", (event) => {
    if (!canHandleKeyboardShortcut(event)) return;

    let button = null;
    if (event.key === "ArrowLeft") {
      button = document.querySelector(".nav-btn.left:not(.disabled)");
    } else if (event.key === "ArrowRight") {
      button = document.querySelector(".nav-btn.right:not(.disabled)");
    } else if (event.key === "t" || event.key === "T") {
      button = document.querySelector(".today-btn");
    }

    if (button) {
      event.preventDefault();
      button.click();
    }
  });

  // Listen to Datastar signal patches
  document.addEventListener("datastar-signal-patch", (e) => {
    const signals = e.detail;
    console.debug("[Signals] Received:", Object.keys(signals).join(", "));
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
(function () {
  "use strict";

  let eventSource = null;
  let shutdownOverlay = null;
  let isConnected = false;
  let healthCheckTimer = null;
  let initialized = false;
  let unloading = false;

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

  function applyPatchElements(elements) {
    document.dispatchEvent(
      new CustomEvent("datastar-fetch", {
        detail: {
          type: "datastar-patch-elements",
          el: document.documentElement,
          argsRaw: { elements },
        },
      }),
    );
  }

  function applyPatchSignals(json) {
    try {
      mergePatch(JSON.parse(json));
    } catch (error) {
      console.error("[SSE] Error parsing signals:", error);
    }
  }

  function showShutdownOverlay() {
    if (shutdownOverlay) return;
    console.log("[SSE] 🎭 Creating shutdown overlay...");
    const overlay = document.createElement("div");
    overlay.id = "shutdown-overlay";
    overlay.innerHTML = `<div class="realm-shutdown">
      <div class="realm-content">
        <h1>⚔️ THE REALM REBIRTHS ⚔️</h1>
        <span class="skull-icon">💀</span>
        <p>The Quest Log realm is undergoing mystical regeneration...</p>
        <p class="sub-message">Thy progress is safe. Return shortly, brave adventurer.</p>
        <div class="progress-bar">
          <div class="progress-bar-fill"></div>
        </div>
        <div class="retry-dots">
          <div class="retry-dot"></div>
          <div class="retry-dot"></div>
          <div class="retry-dot"></div>
        </div>
        <p class="tip">Waiting for realm to revive...</p>
      </div>
    </div>`;
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

    // Check every 5 seconds. EventSource covers initial connectivity.
    healthCheckTimer = setInterval(check, 5000);
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
      if (unloading || document.visibilityState === "hidden") {
        eventSource.close();
        return;
      }
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

  function initApp() {
    if (initialized) return;
    initialized = true;

    connect();
    // Start proactive health check to detect server death even if EventSource doesn't fire onerror
    startProactiveHealthCheck();
    // Initialize video modal and quest UI
    initVideoModal();
    initQuestUI();
  }

  // Handle both cases: DOMContentLoaded already fired or not yet
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initApp);
  } else {
    // DOMContentLoaded has already fired, initialize immediately
    initApp();
  }

  globalThis.addEventListener("pagehide", function () {
    unloading = true;
    isConnected = false;
    stopProactiveHealthCheck();
    eventSource?.close();
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
