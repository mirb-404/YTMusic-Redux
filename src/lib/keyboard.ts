import { invoke } from "@tauri-apps/api/core";

// In-app keyboard controls. The key map and its actions live in Rust (keyboard.rs); the YTM
// bridge normalizes and matches keys the same way, so both views behave alike.

export type Binding = { keys: string[]; action: string; description: string };

// Held keys repeat these; every other action fires once per press.
const REPEATING = new Set(["volumeUp", "volumeDown", "seekForward", "seekBack"]);

/** Modifiers in the order Ctrl, Alt, Shift, then the key; Shift only for letters and named keys. */
export function combo(e: KeyboardEvent): string {
  const key = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const shift = e.shiftKey && (e.key.length > 1 || /^[a-z]$/i.test(e.key));
  return [e.ctrlKey && "Ctrl", e.altKey && "Alt", shift && "Shift", key].filter(Boolean).join("+");
}

/** Typing in a field, or Space on a focused control, keeps its usual meaning. */
function ignored(e: KeyboardEvent, keys: string): boolean {
  const target = e.target as HTMLElement | null;
  if (target?.closest?.("input:not([type=range]):not([type=checkbox]), textarea, select, [contenteditable]")) {
    return !(e.ctrlKey || e.altKey || /^F\d+$/.test(e.key));
  }
  return keys === "Space" && !!target?.closest?.("button, a, input, [role=button], [role=slider]");
}

export const keyboardBindings = () => invoke<Binding[]>("keyboard_bindings");

export async function keyboardControls(): Promise<(e: KeyboardEvent) => void> {
  const bindings = await keyboardBindings();
  const actions = new Map(bindings.flatMap((b) => b.keys.map((k) => [k, b.action] as const)));
  return (e) => {
    if (e.defaultPrevented || e.isComposing) return;
    const keys = combo(e);
    const action = actions.get(keys);
    if (!action || ignored(e, keys) || (e.repeat && !REPEATING.has(action))) return;
    e.preventDefault();
    invoke("keyboard_action", { action });
  };
}
