import { useEffect, useState } from "react";
import { BackgroundLayer } from "./components/BackgroundLayer";
import { Sidebar, type PageId } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toaster } from "./components/Toaster";
import { cmd, isTauri } from "./lib/bridge";
import {
  hydrateAccounts,
  hydrateSettings,
  initEventBridge,
  useAccounts,
  useSettings,
} from "./lib/store";
import type { Bootstrap } from "./lib/types";
import { AccountsPage } from "./pages/AccountsPage";
import { ConsolePage } from "./pages/ConsolePage";
import { SettingsPage } from "./pages/SettingsPage";

export default function App() {
  const [page, setPage] = useState<PageId>("accounts");
  const [version, setVersion] = useState("1.0.0");
  const [ready, setReady] = useState(false);

  useEffect(() => {
    initEventBridge();
    void cmd<Bootstrap>("get_bootstrap").then((b) => {
      hydrateSettings(b.settings);
      hydrateAccounts(b.accounts, b.instances);
      setVersion(b.appVersion);
      setReady(true);
      void useAccounts.getState().refreshPresence();
    });
    const iv = window.setInterval(() => void useAccounts.getState().refreshPresence(), 45000);
    return () => window.clearInterval(iv);
  }, []);

  return (
    <div className="relative flex h-screen w-screen overflow-hidden">
      <BackgroundLayer />

      <Sidebar page={page} onNavigate={setPage} version={version} />

      <div className="relative flex min-w-0 flex-1 flex-col">
        <TitleBar demo={!isTauri} />
        <main className="min-h-0 flex-1">
          {!ready ? (
            <div className="flex h-full items-center justify-center">
              <div className="h-8 w-8 animate-spin rounded-full border-2 border-line border-t-accent" />
            </div>
          ) : (
            <div key={page} className="h-full animate-fadeUp">
              {page === "accounts" && <AccountsPage />}
              {page === "console" && <ConsolePage />}
              {page === "settings" && <SettingsPage />}
            </div>
          )}
        </main>
      </div>

      <Toaster />
      {/* невидимый потребитель настроек для синхронизации языка */}
      <LangSync />
    </div>
  );
}

function LangSync() {
  useSettings((s) => s.settings.language);
  return null;
}
