import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { replaceWithTranslation } from "./lib/commands";
import "./index.css";

interface TranslationPayload {
  original: string;
  translated: string;
  target_lang: string;
  reverse: boolean;
}

const HIDE_DELAY_MS = 6000;

function Toast() {
  const [payload, setPayload] = useState<TranslationPayload | null>(null);

  useEffect(() => {
    const unlistenPromise = listen<TranslationPayload>("show-translation", (event) => {
      setPayload(event.payload);
    });

    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!payload) return;
    const timer = window.setTimeout(() => {
      void getCurrentWebviewWindow().hide();
    }, HIDE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [payload]);

  const close = () => {
    void getCurrentWebviewWindow().hide();
  };

  const copy = async () => {
    if (!payload) return;
    try {
      await navigator.clipboard.writeText(payload.translated);
    } catch {
      // ignore clipboard errors silently
    }
  };

  const replace = async () => {
    if (!payload) return;
    try {
      await replaceWithTranslation(payload.translated);
    } catch {
      // ignore — replace failures shouldn't crash the toast
    }
  };

  if (!payload) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-background text-xs text-muted-foreground">
            Ожидание перевода...
      </div>
    );
  }

  const isError = payload.target_lang === "" && payload.original === "";

  return (
    <div className="flex h-full w-full flex-col gap-2 bg-background p-3 text-foreground shadow-lg">
      <div className="flex items-start justify-between gap-2">
        <div className="text-[11px] uppercase tracking-wide text-muted-foreground">
          {isError
            ? "KeyTweak"
            : payload.reverse
              ? `Обратный → ${payload.target_lang.toUpperCase()}`
              : `Перевод → ${payload.target_lang.toUpperCase()}`}
        </div>
        <button
          type="button"
          onClick={close}
          className="rounded px-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
          aria-label="Close"
        >
          ×
        </button>
      </div>

      {!isError && payload.original && (
        <div className="line-clamp-2 text-xs text-muted-foreground" title={payload.original}>
          {payload.original}
        </div>
      )}

      <div className="line-clamp-4 flex-1 overflow-hidden text-sm leading-snug" title={payload.translated}>
        {payload.translated}
      </div>

      {!isError && (
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={replace}
            className="rounded border border-border bg-foreground px-2 py-1 text-[11px] text-background hover:opacity-90"
          >
            Заменить
          </button>
          <button
            type="button"
            onClick={copy}
            className="rounded border border-border bg-muted px-2 py-1 text-[11px] text-foreground hover:bg-foreground hover:text-background"
          >
            Копировать
          </button>
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Toast />
  </React.StrictMode>,
);
