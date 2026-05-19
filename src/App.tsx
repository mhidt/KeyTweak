import { useCallback, useEffect, useMemo, useState } from "react";
import { getConfig, isAutoStart, setAutoStart, setConfig } from "./lib/commands";
import type { Config } from "./types/config";
import { ApiKeysSettings } from "./components/ApiKeysSettings";
import { AutoReplaceSettings } from "./components/AutoReplaceSettings";
import { CapsLockSettings } from "./components/CapsLockSettings";
import { ExceptionsSettings } from "./components/ExceptionsSettings";
import { GeneralSettings } from "./components/GeneralSettings";
import { PanelShell } from "./components/PanelShell";
import { Sidebar, type TabId } from "./components/Sidebar";
import { TranslateSettings } from "./components/TranslateSettings";

export function App() {
  const [activeTab, setActiveTab] = useState<TabId>("caps");
  const [config, setConfigState] = useState<Config | null>(null);
  const [savedConfig, setSavedConfig] = useState<Config | null>(null);
  const [status, setStatus] = useState("");
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState("");

  const load = useCallback(() => {
    let cancelled = false;
    setLoadError("");

    async function run() {
      try {
        const loaded = await getConfig();
        const autoStart = await isAutoStart().catch(() => loaded.caps_lock.auto_start);
        const merged = { ...loaded, caps_lock: { ...loaded.caps_lock, auto_start: autoStart } };
        if (!cancelled) {
          setConfigState(merged);
          setSavedConfig(merged);
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : String(error);
          setLoadError(message);
        }
      }
    }

    void run();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    return load();
  }, [load]);

  const title = useMemo(() => {
    switch (activeTab) {
      case "caps":
        return "Переключение языка Caps Lock";
      case "autoreplace":
        return "Автозамена";
      case "translate":
        return "Перевод";
      case "exceptions":
        return "Исключения";
      case "api":
        return "API-ключи";
      case "general":
        return "Общие";
    }
  }, [activeTab]);

  const save = async () => {
    if (!config) return;
    setSaving(true);
    setStatus("");
    try {
      await setConfig(config);
      await setAutoStart(config.caps_lock.auto_start);
      setSavedConfig(config);
      setStatus("Сохранено");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  const cancel = () => {
    if (savedConfig) {
      setConfigState(savedConfig);
      setStatus("Изменения отменены");
    }
  };

  const content = () => {
    if (!config) {
      if (loadError) {
        return (
          <div className="flex flex-col items-center justify-center gap-3 px-8 py-6">
            <p className="text-sm text-red-600">{loadError}</p>
            <button
              type="button"
              onClick={load}
              className="rounded border border-border bg-muted px-3 py-1 text-xs text-foreground hover:bg-foreground hover:text-background"
            >
              Повторить
            </button>
          </div>
        );
      }
      return <div className="px-8 py-6 text-sm text-muted-foreground">Загрузка настроек...</div>;
    }

    switch (activeTab) {
      case "caps":
        return <CapsLockSettings config={config} onChange={setConfigState} />;
      case "autoreplace":
        return <AutoReplaceSettings config={config} onChange={setConfigState} />;
      case "translate":
        return <TranslateSettings config={config} onChange={setConfigState} />;
      case "exceptions":
        return <ExceptionsSettings config={config} onChange={setConfigState} />;
      case "api":
        return <ApiKeysSettings config={config} onChange={setConfigState} />;
      case "general":
        return <GeneralSettings config={config} onChange={setConfigState} />;
    }
  };

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar activeTab={activeTab} onChange={setActiveTab} />
      <main className="min-w-0 flex-1">
        {config ? (
          <PanelShell title={title} onSave={save} onCancel={cancel} status={status} saving={saving}>
            {content()}
          </PanelShell>
        ) : (
          content()
        )}
      </main>
    </div>
  );
}

