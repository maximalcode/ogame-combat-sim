import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/App";
// PROTOTYPE — throwaway branch only. Renders the #7 layout variants instead of
// the shell in dev builds. Never merge this import to develop.
import { PrototypePage } from "@/prototype/PrototypePage";
import "@/index.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

createRoot(rootElement).render(
  <StrictMode>{import.meta.env.DEV ? <PrototypePage /> : <App />}</StrictMode>,
);
