// The product name, in one place: the Fluent term `-app-name`
// (crates/app/i18n/*/belisk.ftl). Belisk, after the Kibelisk players
// plant to respawn (decided 2026-10-09; it was "AION 2 DPS Meter"). A rename
// changes only this line. Read by app.js (the main window's title bar),
// hub.js (the toolbar, the prompt, the source lines), todos.js and updates.js
// (Settings › About); load it before the page's own script.
window.APP_NAME = "Belisk";

// Page notes name the app with <span data-app-name>Belisk</span>: the text in
// the source is for reading it, the page shows this constant.
document.addEventListener("DOMContentLoaded", function () {
  document.querySelectorAll("[data-app-name]").forEach(function (el) {
    el.textContent = window.APP_NAME;
  });
});
