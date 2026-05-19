import { getPanelForApp, hasPanel } from "@/components/panels/panelRegistry";
import React from "react";
import { render } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

describe("panelRegistry — World Monitor integration", () => {
  test("registers world-monitor in PANEL_MAP", () => {
    expect(hasPanel("world-monitor")).toBe(true);
  });

  test("getPanelForApp returns a React node for world-monitor", () => {
    const node = getPanelForApp("world-monitor");
    // render should not throw — component may lazy-load an iframe
    expect(() => render(<>{node}</>)).not.toThrow();
  });

  test("does not define duplicate literal app IDs", () => {
    const testDir = dirname(fileURLToPath(import.meta.url));
    const registryPath = resolve(testDir, "../../src/components/panels/panelRegistry.tsx");
    const registrySource = readFileSync(registryPath, "utf8");
    const seen = new Map<string, number>();
    const duplicates: string[] = [];

    registrySource.split(/\r?\n/).forEach((line, index) => {
      const match = line.match(/^\s*"([^"]+)"\s*:/);
      if (!match) return;

      const key = match[1];
      const firstLine = seen.get(key);
      if (firstLine) {
        duplicates.push(`${key} first defined at ${firstLine}, repeated at ${index + 1}`);
        return;
      }

      seen.set(key, index + 1);
    });

    expect(duplicates).toEqual([]);
  });
});
