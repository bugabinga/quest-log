(function () {
  "use strict";

  const STORAGE_KEY = "quest-log-first-time";

  const modal = document.getElementById("video-modal");
  const video = document.getElementById("intro-video");
  const closeBtn = document.querySelector(".video-close");

  function openVideoModal() {
    if (!modal || !video) return;
    modal.style.display = "flex";
    modal.style.opacity = "1";
    video.currentTime = 0;
    video.play().catch(function () {});
  }

  function closeVideoModal() {
    if (!modal || !video) return;
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

  if (!modal || !video) return;

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
})();
