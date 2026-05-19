/**
 * useApplicationRegistry — fetches and caches the application registry.
 */
import { useEffect } from "react";
import { useApplicationStore } from "@/stores/applicationStore";
import {
  fetchApplicationRegistry,
  DEFAULT_APPLICATIONS,
} from "@/services/applicationService";
import { APP_STORE_APPS } from "@/config/appstore.config";
import type { Application } from "@/types/application";

// Map a frontend App Store manifest into an `Application` entry usable by the desktop registry
function mapAppStoreToApplication(appStoreEntry: any): Application {
  const id = (appStoreEntry.id || appStoreEntry.name || appStoreEntry.repositoryUrl || '')
    .toString()
    .toLowerCase()
    .replace(/[^a-z0-9-_]/g, '-')
    .replace(/-+/g, '-');

  const launchIsTauri = String(appStoreEntry.launchCommand || '').toLowerCase().includes('tauri') || (appStoreEntry.requirements || []).some((r: string) => /tauri/i.test(r));

  return {
    id,
    name: appStoreEntry.name || id,
    description: appStoreEntry.description || '',
    category: (appStoreEntry.category || 'utility') as any,
    preinstalled: !!appStoreEntry.installed,
    icon: { type: 'placeholder', category: (appStoreEntry.category || 'other'), color: '#888' },
    launchCommand: launchIsTauri ? { type: 'tauri', target: `launch_${id}` } : { type: 'process', target: (appStoreEntry.launchCommand || appStoreEntry.launch || 'npm run start') },
  } as Application;
}

/**
 * Fetches the application registry on mount and populates the store.
 */
export function useApplicationRegistry(): void {
  const setApplications = useApplicationStore((s) => s.setApplications);
  const applications = useApplicationStore((s) => s.applications);

  useEffect(() => {
    if (applications.length > 0) return; // already loaded

    let cancelled = false;

    fetchApplicationRegistry().then((apps) => {
      if (!cancelled) {
        // Convert installed App Store Tauri apps into desktop `Application` entries and merge them
        const preinstalledFromStore = APP_STORE_APPS
          .filter(a => a.installed)
          .map(mapAppStoreToApplication);

        const base = apps.length > 0 ? apps : DEFAULT_APPLICATIONS;

        // Merge by id (desktop default registry takes precedence) and keep stable order
        const mergedById = new Map<string, Application>();
        base.forEach(a => mergedById.set(a.id, a));
        preinstalledFromStore.forEach(a => {
          if (!mergedById.has(a.id)) {
            mergedById.set(a.id, a);
          }
        });

        setApplications(Array.from(mergedById.values()));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [setApplications, applications.length]);
}
