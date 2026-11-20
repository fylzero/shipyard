// Suppress the webview's default context menu (Reload / Inspect Element)
// everywhere a component hasn't defined its own. Custom menus still open
// because their handlers run before this bubbles to the window. Editable
// fields keep the native menu so cut/copy/paste and spellcheck still work.
function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return target.closest('input, textarea, [contenteditable=""], [contenteditable="true"]') !== null;
}

window.addEventListener("contextmenu", (event) => {
  if (event.defaultPrevented || isEditable(event.target)) return;
  event.preventDefault();
});
