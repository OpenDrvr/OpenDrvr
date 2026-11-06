import { useState } from "react";
import { AboutDialog } from "./components/layout/AboutDialog";
import { Sidebar } from "./components/layout/Sidebar";
import { ToastProvider } from "./components/ui";
import { useAppNavigation } from "./lib/useAppNavigation";
import { useBackendSnapshot } from "./lib/useBackendSnapshot";
import { DownloadsPage } from "./pages/DownloadsPage";
import { DriversPage } from "./pages/DriversPage";
import { HardwarePage } from "./pages/HardwarePage";
import { OverviewPage } from "./pages/OverviewPage";
import { SettingsPage } from "./pages/SettingsPage";

export default function App() {
  const { page, navigate } = useAppNavigation();
  const { state: backend, refresh } = useBackendSnapshot();
  const [aboutOpen, setAboutOpen] = useState(false);

  const content = (() => {
    switch (page) {
      case "hardware":
        return <HardwarePage />;
      case "drivers":
        return <DriversPage />;
      case "downloads":
        return <DownloadsPage />;
      case "settings":
        return (
          <SettingsPage
            backend={backend}
            onRefresh={() => void refresh()}
            onAbout={() => setAboutOpen(true)}
          />
        );
      case "overview":
      default:
        return (
          <OverviewPage backend={backend} onRefresh={() => void refresh()} onNavigate={navigate} />
        );
    }
  })();

  return (
    <ToastProvider>
      <div className="flex h-screen min-h-0 overflow-hidden bg-surface-950 text-slate-200">
        <Sidebar
          page={page}
          onNavigate={navigate}
          backend={backend}
          onRefresh={() => void refresh()}
          onAbout={() => setAboutOpen(true)}
        />

        <main className="min-w-0 flex-1 overflow-y-auto overflow-x-hidden [scrollbar-gutter:stable]">
          <div className="mx-auto w-full max-w-[1120px] px-6 py-7 lg:px-8 lg:py-8">{content}</div>
        </main>
      </div>

      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} backend={backend} />
    </ToastProvider>
  );
}
