import {
  createParticles,
  createParticlesUncomplete,
  pulseCounter,
  triggerQuestUncompletionNotification,
  triggerSuccessNotification,
} from "./counter-animation.js";

let previousExpToday = 0;

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

const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    if (mutation.type === "attributes" && mutation.attributeName) {
      const el = mutation.target;
      const attr = el.getAttribute(mutation.attributeName);

      if (mutation.attributeName === "data-just-completed" && attr === "true") {
        const button = el.querySelector(".toggle-btn");
        if (button) {
          const rect = button.getBoundingClientRect();
          createParticles(
            rect.left + rect.width / 2,
            rect.top + rect.height / 2,
          );
        }
        triggerSuccessNotification("Quest completed! +XP");
        el.removeAttribute("data-just-completed");
      } else if (
        mutation.attributeName === "data-just-uncompleted" &&
        attr === "true"
      ) {
        const button = el.querySelector(".toggle-btn");
        if (button) {
          const rect = button.getBoundingClientRect();
          createParticlesUncomplete(
            rect.left + rect.width / 2,
            rect.top + rect.height / 2,
          );
        }
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

document.addEventListener("datastar-signal-patch", (e) => {
  const signals = e.detail;
  if (signals.error) {
    import("./counter-animation.js").then((m) =>
      m.triggerErrorNotification(signals.error)
    );
  }

  if (typeof signals.expToday === "number") {
    const expCounter = document.getElementById("exp-counter");
    if (expCounter && signals.expToday !== previousExpToday) {
      const diff = signals.expToday - previousExpToday;
      if (diff !== 0) {
        pulseCounter(expCounter);
      }
    }
    previousExpToday = signals.expToday;
  }
});

addEventListener("popstate", () => {
  location.reload();
});
