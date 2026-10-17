import "./app.css";
import { mount, type Component } from "svelte";
import Shell from "./views/Shell.svelte";
import Settings from "./views/Settings.svelte";
import Mini from "./views/Mini.svelte";
import Auth from "./views/Auth.svelte";

const views: Record<string, Component> = { shell: Shell, settings: Settings, mini: Mini, auth: Auth };
const view = new URLSearchParams(location.search).get("view") ?? "shell";
document.documentElement.dataset.view = view;

// The app has no browser-style context menu; text fields keep theirs.
document.addEventListener("contextmenu", (e) => {
  if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
});

mount(views[view] ?? Shell, { target: document.getElementById("app")! });
