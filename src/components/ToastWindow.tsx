import { Copy, CornerDownLeft, X } from "lucide-react";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { copyToClipboard, hideTranslationToast, replaceWithTranslation } from "../lib/commands";
import { Button } from "./ui/button";

interface TranslationPayload {
  original: string;
  translated: string;
  source_lang: string;
  target_lang: string;
  reverse: boolean;
}

const EMPTY_PAYLOAD: TranslationPayload = {
  original: "",
  translated: "",
  source_lang: "",
  target_lang: "",
  reverse: false,
};

function languageLabel(value: string) {
  if (value.toLowerCase() === "ru") return "RU";
  if (value.toLowerCase() === "en") return "EN";
  return value.toUpperCase();
}

export function ToastWindow() {
  const [payload, setPayload] = useState<TranslationPayload>(EMPTY_PAYLOAD);
  const [status, setStatus] = useState("");

  useEffect(() => {
    let timeoutId: number | undefined;
    let cleanup: (() => void) | undefined;

    listen<TranslationPayload>("show-translation", (event) => {
      setPayload(event.payload);
      setStatus("");

      if (timeoutId) window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        void hideTranslationToast();
      }, 15_000);
    }).then((unlisten) => {
      cleanup = unlisten;
    });

    return () => {
      cleanup?.();
      if (timeoutId) window.clearTimeout(timeoutId);
    };
  }, []);

  const isError = !payload.original;
  const direction = payload.source_lang && payload.target_lang
    ? `${languageLabel(payload.source_lang)} -> ${languageLabel(payload.target_lang)}`
    : "Translation";

  const copy = async () => {
    try {
      await copyToClipboard(payload.translated);
      setStatus("Copied");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  };

  const replace = async () => {
    try {
      await replaceWithTranslation(payload.translated);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="h-screen bg-background text-foreground">
      <div className="flex h-full flex-col rounded-lg border border-border bg-background p-4 shadow-xl">
        <div className="mb-3 flex items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold">{isError ? "Translation failed" : direction}</div>
            {payload.reverse ? <div className="text-xs text-muted-foreground">Reverse translate</div> : null}
          </div>
          <Button variant="ghost" size="icon" onClick={hideTranslationToast} aria-label="Close">
            <X size={16} />
          </Button>
        </div>

        <div className="min-h-0 flex-1 space-y-3 overflow-hidden">
          {!isError ? (
            <div className="max-h-16 overflow-hidden rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
              {payload.original}
            </div>
          ) : null}
          <div className="max-h-24 overflow-auto rounded-md border border-border px-3 py-2 text-sm leading-5">
            {payload.translated || "Waiting for translation..."}
          </div>
        </div>

        <div className="mt-3 flex items-center justify-between gap-2">
          <div className="min-w-0 truncate text-xs text-muted-foreground">{status}</div>
          <div className="flex shrink-0 gap-2">
            {!isError ? (
              <Button variant="outline" size="sm" onClick={replace}>
                <CornerDownLeft size={14} /> Replace
              </Button>
            ) : null}
            <Button size="sm" onClick={copy} disabled={!payload.translated}>
              <Copy size={14} /> Copy
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
