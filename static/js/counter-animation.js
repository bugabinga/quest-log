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

export function createParticles(x, y, color = "#5466ff") {
  const colors = [color, "#33ff99", "#ffcc00", "#ff6699"];

  for (let i = 0; i < 8; i++) {
    const particle = document.createElement("div");
    particle.className = "celebration-particle";
    particle.style.left = x + "px";
    particle.style.top = y + "px";
    particle.style.background = colors[i % colors.length];

    const angle = (i / 8) * Math.PI * 2;
    const distance = 50 + Math.random() * 50;
    const tx = Math.cos(angle) * distance;
    const ty = Math.sin(angle) * distance;
    particle.style.setProperty("--tx", tx + "px");
    particle.style.setProperty("--ty", ty + "px");

    document.body.appendChild(particle);
    setTimeout(() => particle.remove(), 1000);
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

export function createParticlesUncomplete(x, y) {
  const colors = ["#6b7280", "#9ca3af", "#4b5563", "#374151"];

  for (let i = 0; i < 6; i++) {
    const particle = document.createElement("div");
    particle.className = "celebration-particle";
    particle.style.left = x + "px";
    particle.style.top = y + "px";
    particle.style.background = colors[i % colors.length];

    const angle = (i / 6) * Math.PI * 2 + Math.PI;
    const distance = 30 + Math.random() * 30;
    const tx = Math.cos(angle) * distance;
    const ty = Math.sin(angle) * distance;
    particle.style.setProperty("--tx", tx + "px");
    particle.style.setProperty("--ty", Math.abs(ty) + "px");

    document.body.appendChild(particle);
    setTimeout(() => particle.remove(), 1000);
  }
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
