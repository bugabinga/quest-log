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
  let reconnectAttempts = 0;
  const maxReconnectAttempts = 10;
  const baseReconnectDelay = 1000;

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

    eventSource.onopen = function () {
      console.log("[SSE] Connected to /events");
      reconnectAttempts = 0;
    };

    eventSource.onerror = function (err) {
      console.error("[SSE] Error:", err);
      eventSource.close();

      if (reconnectAttempts < maxReconnectAttempts) {
        const delay = baseReconnectDelay * Math.pow(2, reconnectAttempts);
        console.log(
          `[SSE] Reconnecting in ${delay}ms (attempt ${reconnectAttempts + 1})`,
        );
        setTimeout(connect, delay);
        reconnectAttempts++;
      } else {
        console.error("[SSE] Max reconnect attempts reached");
      }
    };
  }

  document.addEventListener("DOMContentLoaded", function () {
    connect();
  });

  document.addEventListener("visibilitychange", function () {
    if (
      document.visibilityState === "visible" &&
      (!eventSource || eventSource.readyState === EventSource.CLOSED)
    ) {
      console.log("[SSE] Page visible, reconnecting...");
      reconnectAttempts = 0;
      connect();
    }
  });
})();
