import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App } from "./App";
import { ToastWindow } from "./components/ToastWindow";
import "./index.css";

const isTauri = typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
const Root = isTauri && getCurrentWindow().label === "toast" ? ToastWindow : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
