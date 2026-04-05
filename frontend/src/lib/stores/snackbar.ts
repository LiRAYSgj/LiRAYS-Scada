import { writable } from "svelte/store";

export type SnackbarType = "success" | "warning" | "error";

export interface SnackbarMessage {
  message: string;
  type: SnackbarType;
  /** Visibility duration in ms; default 5000 */
  duration?: number;
}

const DEFAULT_DURATION_MS = 5_000;

function createSnackbarStore() {
  const { subscribe, set } = writable<SnackbarMessage | null>(null);
  const log = (level: SnackbarType, message: string): void => {
    console.log(`[snackbar:${level}] ${message}`);
  };

  return {
    subscribe,
    show(payload: SnackbarMessage | string): void {
      const entry: SnackbarMessage =
        typeof payload === "string"
          ? { message: payload, type: "error", duration: DEFAULT_DURATION_MS }
          : {
              message: payload.message,
              type: payload.type,
              duration: payload.duration ?? DEFAULT_DURATION_MS,
            };
      log(entry.type, entry.message);
      set(entry);
    },
    success(message: string, duration?: number): void {
      log("success", message);
      set({
        message,
        type: "success",
        duration: duration ?? DEFAULT_DURATION_MS,
      });
    },
    warning(message: string, duration?: number): void {
      log("warning", message);
      set({
        message,
        type: "warning",
        duration: duration ?? DEFAULT_DURATION_MS,
      });
    },
    error(message: string, duration?: number): void {
      log("error", message);
      set({
        message,
        type: "error",
        duration: duration ?? DEFAULT_DURATION_MS,
      });
    },
    hide(): void {
      set(null);
    },
  };
}

export const snackbarStore = createSnackbarStore();
