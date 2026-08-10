// PROTOTYPE — throwaway. Switcher for the #7 layout variants.
//
// Three variants of the whole page, switchable via ?variant= on the app's only
// route. Read-only demo data throughout — nothing here talks to the API. The
// floating pill is deliberately ugly-distinct so nobody evaluates it as part
// of any design, and the whole page is gated to dev builds.

import { useEffect, useState } from "react";
import { API_BASE_URL, isSameOrigin } from "@/config";
import { VariantA } from "./VariantA";
import { VariantB } from "./VariantB";
import { VariantC } from "./VariantC";

const VARIANTS = [
  { key: "A", name: "Form flow", component: VariantA },
  { key: "B", name: "Versus board", component: VariantB },
  { key: "C", name: "Cockpit", component: VariantC },
] as const;

type VariantKey = (typeof VARIANTS)[number]["key"];

function readVariant(): VariantKey {
  const v = new URLSearchParams(window.location.search).get("variant");
  return VARIANTS.some((x) => x.key === v) ? (v as VariantKey) : "A";
}

function isTyping(): boolean {
  const el = document.activeElement;
  return (
    el instanceof HTMLInputElement ||
    el instanceof HTMLTextAreaElement ||
    (el instanceof HTMLElement && el.isContentEditable)
  );
}

export function PrototypePage() {
  const [variant, setVariant] = useState<VariantKey>(readVariant);

  const go = (dir: 1 | -1) => {
    const i = VARIANTS.findIndex((v) => v.key === variant);
    const next = VARIANTS[(i + dir + VARIANTS.length) % VARIANTS.length] ?? VARIANTS[0];
    const url = new URL(window.location.href);
    url.searchParams.set("variant", next.key);
    window.history.replaceState(null, "", url);
    setVariant(next.key);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTyping()) return;
      if (e.key === "ArrowRight") go(1);
      if (e.key === "ArrowLeft") go(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  });

  const current = VARIANTS.find((v) => v.key === variant) ?? VARIANTS[0];
  const Body = current.component;

  return (
    <div className="min-h-screen bg-slate-950 pb-16 text-slate-100">
      <header className="border-b border-slate-800 bg-slate-900/60">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          <h1 className="text-lg font-semibold">OGame Combat Simulator</h1>
          <span className="font-mono text-xs text-slate-500">
            API: {isSameOrigin ? "same-origin" : API_BASE_URL}
          </span>
        </div>
      </header>

      <Body />

      {import.meta.env.DEV && (
        <div className="fixed bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-3 rounded-full border-2 border-amber-500 bg-slate-900 px-4 py-2 shadow-lg">
          <button
            onClick={() => {
              go(-1);
            }}
            className="text-amber-400"
          >
            ←
          </button>
          <span className="text-xs font-semibold text-amber-300">
            {current.key} — {current.name}
          </span>
          <button
            onClick={() => {
              go(1);
            }}
            className="text-amber-400"
          >
            →
          </button>
        </div>
      )}
    </div>
  );
}
