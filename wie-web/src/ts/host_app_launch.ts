// Kept in the web layer so upstream emulator-core updates remain easy to merge.
// Same URL contract as the existing MRP runner: `f` for a game file,
// `res` for an archive. `app_url`/`game_url` are accepted from the site runner.
import { getLocale, t } from "./i18n";
import type { LibraryApp, Wie } from "./backend";

function readLaunchUrl(): string | undefined {
  const params = new URLSearchParams(window.location.search);
  return params.get("f") ?? params.get("res") ?? params.get("app_url") ?? params.get("game_url") ?? undefined;
}

function inferFilename(url: string): string {
  try {
    const basename = decodeURIComponent(new URL(url, window.location.href).pathname.split("/").pop() ?? "");
    if (/\.(zip|jar)$/i.test(basename)) return basename;
  } catch {
    // The title query parameter is used as a fallback below.
  }
  const title = new URLSearchParams(window.location.search).get("title")?.trim();
  return title ? `${title}.zip` : "game.zip";
}

async function fetchArchive(url: string): Promise<File> {
  const absolute = new URL(url, window.location.href).href;
  const response = await fetch(absolute);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const bytes = await response.arrayBuffer();
  if (bytes.byteLength === 0) throw new Error("Empty archive");
  return new File([bytes], inferFilename(url), { type: "application/zip" });
}

function duplicateTitle(error: unknown): string | undefined {
  const message = error instanceof Error ? error.message : String(error);
  const prefixes: Record<string, string> = {
    "zh-CN": "游戏库中已有此应用：",
    ko: "이미 라이브러리에 추가된 앱입니다: ",
    en: "This app is already in the library: ",
  };
  const prefix = prefixes[getLocale()];
  return message.startsWith(prefix) ? message.slice(prefix.length) : undefined;
}

export async function launchHostApp(backend: Wie, routeToApp: (app: LibraryApp) => Promise<void>, url = readLaunchUrl()): Promise<void> {
  if (!url) return;

  const dialog = document.getElementById("host-launch-dialog") as HTMLDialogElement | null;
  const status = document.getElementById("host-launch-status") as HTMLElement | null;
  const closeButton = document.getElementById("host-launch-close") as HTMLButtonElement | null;
  const close = () => dialog?.close();
  closeButton?.addEventListener("click", close, { once: true });
  dialog?.showModal();

  try {
    const file = await fetchArchive(url);
    try {
      await backend.importApp(file);
    } catch (error) {
      // The library only exposes an ID collision; locate the old record by title or filename.
      const title = duplicateTitle(error);
      const existingApps = await backend.listApps();
      const existing = existingApps.find(item => item.title === title) ??
        existingApps.find(item => item.filename === file.name);
      if (!existing) throw error;
      await backend.deleteApp(existing.id);
      await backend.importApp(file);
    }
    const apps = await backend.listApps();
    const app = apps.find(item => item.filename === file.name) ?? apps.at(-1);
    if (!app) throw new Error(t("invalidArchive"));
    if (status) status.textContent = t("hostLaunchStarting");
    dialog?.close();
    await routeToApp(app);
  } catch (error) {
    if (status) {
      status.textContent = t("hostLaunchFailed", { error: String(error) });
      status.classList.add("error");
      status.removeAttribute("data-i18n-text");
    }
    console.error("Host app launch failed", error);
  } finally {
    if (!status?.classList.contains("error")) close();
  }
}
