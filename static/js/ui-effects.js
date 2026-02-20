export function pulseCounter(element) {
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

export function triggerErrorNotification(message) {
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

export function triggerSuccessNotification(message) {
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

export function triggerQuestUncompletionNotification() {
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

export function triggerRewardClaimedNotification(rewardId) {
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
