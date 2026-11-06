import {
  Download,
  LayoutDashboard,
  MonitorCog,
  ScanSearch,
  Settings,
  type LucideIcon,
} from "lucide-react";

export type PageId = "overview" | "hardware" | "drivers" | "downloads" | "settings";

export interface PageDefinition {
  id: PageId;
  label: string;
  description: string;
  icon: LucideIcon;
}

export interface NavSection {
  label: string;
  pages: readonly PageDefinition[];
}

export const NAV_SECTIONS: readonly NavSection[] = [
  {
    label: "Workspace",
    pages: [
      {
        id: "overview",
        label: "Overview",
        description: "Application status and what OpenDrvr is building toward.",
        icon: LayoutDashboard,
      },
      {
        id: "hardware",
        label: "Hardware",
        description: "Components detected on this computer.",
        icon: MonitorCog,
      },
      {
        id: "drivers",
        label: "Drivers",
        description: "Compatible drivers from official manufacturer sources.",
        icon: ScanSearch,
      },
      {
        id: "downloads",
        label: "Downloads",
        description: "Driver packages and transfer history.",
        icon: Download,
      },
    ],
  },
  {
    label: "Application",
    pages: [
      {
        id: "settings",
        label: "Settings",
        description: "Preferences, local storage and application details.",
        icon: Settings,
      },
    ],
  },
];

export const PAGES: readonly PageDefinition[] = NAV_SECTIONS.flatMap((section) => section.pages);

export function isPageId(value: string): value is PageId {
  return PAGES.some((page) => page.id === value);
}

export function readPageFromHash(): PageId {
  const value = window.location.hash.replace(/^#\/?/, "").trim();
  return isPageId(value) ? value : "overview";
}
