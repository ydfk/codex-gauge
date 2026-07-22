import { mount } from "svelte";
import App from "./app/App.svelte";
import "./app/styles.css";
import "./app/mac-menu.css";

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
