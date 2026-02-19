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
