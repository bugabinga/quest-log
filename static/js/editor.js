// Editor JavaScript - Handles tab switching, form submissions, and UI interactions

// Initialize when DOM is ready
document.addEventListener("DOMContentLoaded", function () {
  console.log("Editor JS loaded");

  // Initialize signals for the editor if not already set
  globalThis.DatastarSignals = globalThis.DatastarSignals || {};

  // Set default signals if they don't exist
  if (typeof globalThis.DatastarSignals._activeTab === "undefined") {
    globalThis.DatastarSignals._activeTab = "quests";
  }
  if (typeof globalThis.DatastarSignals._showQuestForm === "undefined") {
    globalThis.DatastarSignals._showQuestForm = false;
  }
  if (typeof globalThis.DatastarSignals._showRewardForm === "undefined") {
    globalThis.DatastarSignals._showRewardForm = false;
  }
});

// Tab switching functions
function switchTab(tabName) {
  console.log("Switching to tab:", tabName);
  // Hide all panels
  document.querySelectorAll(".editor-panel").forEach((panel) => {
    panel.classList.add("hidden");
  });

  // Show selected panel
  const selectedPanel = document.querySelector(`#${tabName}-panel`);
  if (selectedPanel) {
    selectedPanel.classList.remove("hidden");
  }

  // Update tab active states
  document.querySelectorAll(".editor-tab").forEach((tab) => {
    tab.classList.remove("editor-tab--active");
  });

  const activeTab = document.querySelector(
    `.editor-tab[data-tab="${tabName}"]`,
  );
  if (activeTab) {
    activeTab.classList.add("editor-tab--active");
  }

  // Update signals
  if (globalThis.DatastarSignals) {
    globalThis.DatastarSignals._activeTab = tabName;
  }
}

// Toggle quest form visibility
function toggleQuestForm() {
  const formContainer = document.getElementById("quest-form-container");
  if (formContainer) {
    formContainer.classList.toggle("hidden");
  }
  if (globalThis.DatastarSignals) {
    globalThis.DatastarSignals._showQuestForm = !globalThis.DatastarSignals
      ._showQuestForm;
  }
}

// Toggle reward form visibility
function toggleRewardForm() {
  const formContainer = document.getElementById("reward-form-container");
  if (formContainer) {
    formContainer.classList.toggle("hidden");
  }
  if (globalThis.DatastarSignals) {
    globalThis.DatastarSignals._showRewardForm = !globalThis.DatastarSignals
      ._showRewardForm;
  }
}

// Image preview functionality
function previewImage(input, _previewId) {
  const file = input.files[0];
  if (!file) return;

  // Check file size (max 5MB)
  if (file.size > 5 * 1024 * 1024) {
    showToast("Image must be less than 5MB", "error");
    input.value = "";
    return;
  }

  // Check file type
  if (!file.type.startsWith("image/")) {
    showToast("Please select an image file", "error");
    input.value = "";
    return;
  }

  const reader = new FileReader();
  reader.onload = function () {
    // Could show preview if we have a preview element
    console.log("Image selected:", file.name);
  };
  reader.readAsDataURL(file);
}

// Delete confirmation
function confirmDelete(type, id) {
  const confirmed = confirm(
    `Are you sure you want to delete this ${type}? This action cannot be undone.`,
  );
  if (confirmed) {
    console.log(`Deleting ${type}:`, id);
    // The actual deletion is handled by Datastar's data-on:click handler
  } else {
    // Prevent the default action
    event.preventDefault();
    event.stopPropagation();
  }
}

// Toast notification system
function showToast(message, type = "success") {
  // Remove existing toasts
  const existingToasts = document.querySelectorAll(".toast");
  existingToasts.forEach((toast) => toast.remove());

  // Create toast element
  const toast = document.createElement("div");
  toast.className = `toast toast--${type}`;
  toast.textContent = message;

  // Add to container
  let container = document.getElementById("toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "toast-container";
    container.className = "toast-container";
    document.body.appendChild(container);
  }

  container.appendChild(toast);

  // Auto-remove after 3 seconds
  setTimeout(() => {
    toast.style.animation = "toast-slide-in 0.3s ease-out reverse";
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// Handle form submission feedback
document.addEventListener("submit", function (e) {
  const form = e.target;

  // Check if it's an editor form
  if (
    form.id === "quest-form" || form.id === "reward-form" ||
    form.id === "settings-form"
  ) {
    console.log("Form submitted:", form.id);

    // The actual submission is handled by Datastar
    // This is just for additional custom handling if needed
  }
});

// Handle file input changes
document.addEventListener("change", function (e) {
  if (e.target.type === "file") {
    previewImage(e.target);
  }
});

// Handle Datastar signal updates
if (typeof globalThis.Datastar !== "undefined") {
  globalThis.Datastar.onSignalsUpdate(function (signals) {
    console.log("Signals updated:", signals);

    // Handle specific signal changes
    if (signals.questSaved) {
      showToast("Quest saved successfully!", "success");
    }

    if (signals.rewardSaved) {
      showToast("Reward saved successfully!", "success");
    }

    if (signals.settingsSaved) {
      showToast("Settings saved successfully!", "success");
    }

    if (signals.loginError) {
      showToast(signals.loginError, "error");
    }

    if (signals.questDeleted) {
      showToast("Quest deleted", "success");
    }

    if (signals.rewardDeleted) {
      showToast("Reward deleted", "success");
    }

    // Update local signal state
    if (signals._activeTab) {
      switchTab(signals._activeTab);
    }
  });
}

// Make functions globally available
globalThis.switchTab = switchTab;
globalThis.toggleQuestForm = toggleQuestForm;
globalThis.toggleRewardForm = toggleRewardForm;
globalThis.previewImage = previewImage;
globalThis.confirmDelete = confirmDelete;
globalThis.showToast = showToast;

console.log("Editor JavaScript initialized");
