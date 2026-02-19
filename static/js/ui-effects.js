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
