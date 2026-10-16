/**
 * @module ui/ContextMenu
 * Right-click context menu for scene entities.
 *
 * Positions next to cursor, auto-closes on click-away,
 * supports nested sub-menus.
 */

export interface ContextMenuItem {
  label: string;
  icon?: string;
  disabled?: boolean;
  separator?: boolean;
  action?: () => void;
  children?: ContextMenuItem[];
}

export class ContextMenu {
  private readonly root: HTMLElement;
  private menuEl: HTMLElement | null = null;
  private readonly onOutsideClick: (e: MouseEvent) => void;

  constructor(root: HTMLElement) {
    this.root = root;
    this.onOutsideClick = this.handleOutsideClick.bind(this);
  }

  /** Show the context menu at screen coordinates. */
  show(x: number, y: number, items: ContextMenuItem[]): void {
    this.hide();

    this.menuEl = document.createElement("div");
    this.menuEl.className = "context-menu";
    this.menuEl.innerHTML = this.renderItems(items);

    // Position
    this.menuEl.style.left = `${x}px`;
    this.menuEl.style.top = `${y}px`;

    this.root.appendChild(this.menuEl);

    // Bind actions
    this.menuEl.querySelectorAll<HTMLElement>("[data-idx]").forEach((el) => {
      const idx = parseInt(el.dataset.idx!, 10);
      const item = items[idx];
      if (item.action && !item.disabled) {
        el.addEventListener("click", (e) => {
          e.stopPropagation();
          item.action!();
          this.hide();
        });
      }
    });

    // Ensure menu stays in viewport
    requestAnimationFrame(() => {
      if (!this.menuEl) return;
      const rect = this.menuEl.getBoundingClientRect();
      if (rect.right > window.innerWidth) {
        this.menuEl.style.left = `${window.innerWidth - rect.width - 8}px`;
      }
      if (rect.bottom > window.innerHeight) {
        this.menuEl.style.top = `${window.innerHeight - rect.height - 8}px`;
      }
    });

    // Close on outside click (delayed to avoid immediate close)
    setTimeout(() => {
      window.addEventListener("click", this.onOutsideClick);
    }, 0);
  }

  /** Hide the context menu. */
  hide(): void {
    if (this.menuEl) {
      this.menuEl.remove();
      this.menuEl = null;
    }
    window.removeEventListener("click", this.onOutsideClick);
  }

  /** Is the menu currently visible? */
  isVisible(): boolean {
    return this.menuEl !== null;
  }

  dispose(): void {
    this.hide();
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private renderItems(items: ContextMenuItem[]): string {
    return items
      .map((item, i) => {
        if (item.separator) {
          return '<div class="ctx-separator"></div>';
        }
        const disabled = item.disabled ? "disabled" : "";
        const icon = item.icon ? `<span class="ctx-icon">${item.icon}</span>` : "";
        const arrow = item.children ? '<span class="ctx-arrow">▸</span>' : "";
        return `<div class="ctx-item ${disabled}" data-idx="${i}">${icon}<span class="ctx-label">${item.label}</span>${arrow}</div>`;
      })
      .join("");
  }

  private handleOutsideClick(e: MouseEvent): void {
    if (this.menuEl && !this.menuEl.contains(e.target as Node)) {
      this.hide();
    }
  }
}
