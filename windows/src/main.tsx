import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App } from "./App";
import { SettingsApp } from "./settings/SettingsApp";
import "./styles.css";

// mesmo bundle pras duas janelas; o label decide qual UI montar
const view = getCurrentWindow().label === "settings" ? "settings" : "island";
document.documentElement.dataset.view = view;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{view === "settings" ? <SettingsApp /> : <App />}</React.StrictMode>,
);
