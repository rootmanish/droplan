import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import "@/index.css";

/**
 * Follow the OS appearance. Tauri exposes a theme too, but `matchMedia` is
 * enough here and keeps the startup path free of an await.
 */
function applySystemTheme(matches: boolean) {
  document.documentElement.classList.toggle("dark", matches);
}

const scheme = window.matchMedia("(prefers-color-scheme: dark)");
applySystemTheme(scheme.matches);
scheme.addEventListener("change", (event) => applySystemTheme(event.matches));

const container = document.getElementById("root");
if (!container) throw new Error("missing #root element");

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
