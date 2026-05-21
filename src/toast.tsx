import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { replaceWithTranslation } from "./lib/commands";
import "./index.css";

interface TranslationPayload {
  original: string;
  translated: string;
  source_lang?: string;
  target_lang: string;
  reverse: boolean;
}

interface AppToastPayload {
  title: string;
  message: string;
}

const HIDE_DELAY_MS = 6000;

function Toast() {
  const [payload, setPayload] = useState<TranslationPayload | null>(null);
  const [appToast, setAppToast] = useState<AppToastPayload | null>(null);

  useEffect(() => {
    const unlistenTranslation = listen<TranslationPayload>(
      "show-translation",
      (event) => {
        setAppToast(null);
        setPayload(event.payload);
      },
    );
    const unlistenAppToast = listen<AppToastPayload>(
      "show-app-toast",
      (event) => {
        setPayload(null);
        setAppToast(event.payload);
      },
    );

    return () => {
      void unlistenTranslation.then((fn) => fn());
      void unlistenAppToast.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!payload && !appToast) return;
    const timer = window.setTimeout(
      () => {
        void getCurrentWebviewWindow().hide();
      },
      appToast ? 5000 : HIDE_DELAY_MS,
    );
    return () => window.clearTimeout(timer);
  }, [payload, appToast]);

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
    if (appToast) {
      return (
        <div className="flex h-full w-full flex-col overflow-hidden rounded-md bg-zinc-950 text-white shadow-lg">
          <div className="flex-1 px-4 py-3">
            <div className="flex items-start justify-between gap-3">
              <div className="flex items-center gap-3">
                <div className="flex h-9 w-9 items-center justify-center rounded-md bg-white/10 text-base">
                  ⌨
                </div>
                <div>
                  <div className="flex items-center gap-2 text-sm font-semibold">
                    {appToast.title}
                  </div>
                  <div className="mt-1 text-xs text-zinc-400">
                    {appToast.message}
                  </div>
                </div>
              </div>
              <button
                type="button"
                onClick={close}
                className="rounded px-1.5 py-0.5 text-sm text-zinc-300 hover:bg-white/10 hover:text-white"
                aria-label="Close"
              >
                ×
              </button>
            </div>
          </div>
        </div>
      );
    }

    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-3 bg-background text-xs text-muted-foreground">
        <button
          type="button"
          onClick={close}
          className="absolute right-3 top-2 rounded px-1.5 py-0.5 text-sm hover:bg-muted hover:text-foreground"
          aria-label="Close"
        >
          ×
        </button>
        Ожидание события...
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
        <div
          className="line-clamp-2 text-xs text-muted-foreground"
          title={payload.original}
        >
          {payload.original}
        </div>
      )}

      <div
        className="line-clamp-4 flex-1 overflow-hidden text-sm leading-snug"
        title={payload.translated}
      >
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
