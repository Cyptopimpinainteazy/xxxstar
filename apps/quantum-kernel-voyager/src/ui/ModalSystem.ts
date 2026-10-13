/**
 * @module ui/ModalSystem
 * Centralized modal dialog system.
 *
 * Supports: confirm, alert, custom content, stacked modals.
 * Keyboard: Escape closes topmost modal.
 */

export interface ModalOptions {
  title: string;
  content: string;
  buttons?: ModalButton[];
  closable?: boolean;
  width?: string;
}

export interface ModalButton {
  label: string;
  variant?: "primary" | "secondary" | "danger";
  action: () => void;
}

interface ActiveModal {
  id: string;
  backdrop: HTMLElement;
  dialog: HTMLElement;
  options: ModalOptions;
}

export class ModalSystem {
  private readonly root: HTMLElement;
  private readonly stack: ActiveModal[] = [];
  private readonly onKeyDown: (e: KeyboardEvent) => void;

  constructor(root: HTMLElement) {
    this.root = root;
    this.onKeyDown = this.handleKeyDown.bind(this);
    window.addEventListener("keydown", this.onKeyDown);
  }

  /** Show a modal and return its ID. */
  show(options: ModalOptions): string {
    const id = `modal-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

    const backdrop = document.createElement("div");
    backdrop.className = "modal-backdrop";
    backdrop.dataset.modalId = id;

    const dialog = document.createElement("div");
    dialog.className = "modal-dialog";
    if (options.width) dialog.style.width = options.width;

    dialog.innerHTML = `
      <div class="modal-header">
        <h2 class="modal-title">${options.title}</h2>
        ${options.closable !== false ? '<button class="modal-close">&times;</button>' : ""}
      </div>
      <div class="modal-body">${options.content}</div>
      ${options.buttons ? this.renderButtons(options.buttons, id) : ""}
    `;

    backdrop.appendChild(dialog);
    this.root.appendChild(backdrop);

    // Bind close button
    dialog.querySelector<HTMLButtonElement>(".modal-close")?.addEventListener("click", () => {
      this.close(id);
    });

    // Bind action buttons
    dialog.querySelectorAll<HTMLButtonElement>("[data-modal-action]").forEach((btn, i) => {
      btn.addEventListener("click", () => {
        options.buttons?.[i]?.action();
        this.close(id);
      });
    });

    // Close on backdrop click
    backdrop.addEventListener("click", (e) => {
      if (e.target === backdrop && options.closable !== false) {
        this.close(id);
      }
    });

    // Animate in
    requestAnimationFrame(() => {
      backdrop.classList.add("open");
    });

    this.stack.push({ id, backdrop, dialog, options });
    return id;
  }

  /** Close a specific modal by ID. */
  close(id: string): void {
    const idx = this.stack.findIndex((m) => m.id === id);
    if (idx === -1) return;

    const modal = this.stack[idx];
    modal.backdrop.classList.remove("open");
    setTimeout(() => {
      modal.backdrop.remove();
    }, 200);

    this.stack.splice(idx, 1);
  }

  /** Close all modals. */
  closeAll(): void {
    for (const modal of [...this.stack]) {
      this.close(modal.id);
    }
  }

  /** Convenience: confirm dialog. */
  confirm(title: string, message: string): Promise<boolean> {
    return new Promise((resolve) => {
      this.show({
        title,
        content: `<p>${message}</p>`,
        buttons: [
          { label: "Cancel", variant: "secondary", action: () => resolve(false) },
          { label: "Confirm", variant: "primary", action: () => resolve(true) },
        ],
      });
    });
  }

  /** Convenience: alert dialog. */
  alert(title: string, message: string): Promise<void> {
    return new Promise((resolve) => {
      this.show({
        title,
        content: `<p>${message}</p>`,
        buttons: [{ label: "OK", variant: "primary", action: () => resolve() }],
      });
    });
  }

  dispose(): void {
    this.closeAll();
    window.removeEventListener("keydown", this.onKeyDown);
  }

  // -----------------------------------------------------------------------
  // Private
  // -----------------------------------------------------------------------

  private renderButtons(buttons: ModalButton[], _id: string): string {
    const btns = buttons
      .map(
        (b, i) =>
          `<button class="modal-btn modal-btn-${b.variant ?? "secondary"}" data-modal-action="${i}">${b.label}</button>`,
      )
      .join("");
    return `<div class="modal-footer">${btns}</div>`;
  }

  private handleKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape" && this.stack.length > 0) {
      const top = this.stack[this.stack.length - 1];
      if (top.options.closable !== false) {
        this.close(top.id);
      }
    }
  }
}
