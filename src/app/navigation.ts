import {
  CableIcon,
  FileTextIcon,
  GaugeIcon,
  HammerIcon,
  LayersIcon,
  RocketIcon,
  PackageIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  type LucideIcon,
} from "lucide-react";

import type { PageId } from "@/stores/navigation-store";

export interface NavigationItem {
  id: PageId;
  label: string;
  icon: LucideIcon;
  description: string;
}

export interface NavigationGroup {
  label: string;
  items: NavigationItem[];
}

export const navigationGroups: NavigationGroup[] = [
  {
    label: "Serve",
    items: [
      {
        id: "dashboard",
        label: "Dashboard",
        icon: GaugeIcon,
        description: "Server state, active runtime, and hardware at a glance",
      },
      {
        id: "models",
        label: "Models",
        icon: PackageIcon,
        description: "GGUF models discovered in your model directories",
      },
      {
        id: "profiles",
        label: "Profiles",
        icon: SlidersHorizontalIcon,
        description: "Saved launch configurations for a model and runtime",
      },
      {
        id: "performance",
        label: "Performance",
        icon: RocketIcon,
        description: "Multi-GPU plans and repeatable coding benchmarks",
      },
      {
        id: "agent",
        label: "Agent Connect",
        icon: CableIcon,
        description: "Verify and copy OpenAI-compatible coding-agent settings",
      },
    ],
  },
  {
    label: "llama.cpp",
    items: [
      {
        id: "runtimes",
        label: "Runtimes",
        icon: LayersIcon,
        description: "Sources, versions, and built server runtimes",
      },
      {
        id: "build",
        label: "Build",
        icon: HammerIcon,
        description: "Toolchain detection and CMake build profiles",
      },
    ],
  },
  {
    label: "System",
    items: [
      {
        id: "logs",
        label: "Logs",
        icon: FileTextIcon,
        description: "Raw output from llama-server and build tooling",
      },
      {
        id: "settings",
        label: "Settings",
        icon: SettingsIcon,
        description: "Application paths, Git, build, and server defaults",
      },
    ],
  },
];

export const navigationItems: NavigationItem[] = navigationGroups.flatMap(
  (group) => group.items,
);

export function findNavigationItem(page: PageId): NavigationItem | undefined {
  return navigationItems.find((item) => item.id === page);
}
