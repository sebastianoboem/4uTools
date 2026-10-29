import { api, FileEntry } from "./api";

export const THUMB_MAX_SIDE = 280;
export const LIGHTBOX_MAX_SIDE = 1280;

const IMAGE_EXTS = new Set([
  ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".raw", ".ico", ".heic", ".heif", ".webp",
]);

const previewResolved = new Map<string, string>();
const previewInflight = new Map<string, Promise<string>>();

export function previewKey(path: string, maxSide: number | null) {
  return `${path}|${maxSide ?? 0}`;
}

export function getCachedPreview(path: string, maxSide: number | null) {
  return previewResolved.get(previewKey(path, maxSide)) ?? null;
}

export function getPreview(path: string, maxSide: number | null): Promise<string> {
  const key = previewKey(path, maxSide);
  const cached = previewResolved.get(key);
  if (cached) {
    return Promise.resolve(cached);
  }

  let pending = previewInflight.get(key);
  if (!pending) {
    pending = api
      .getFilePreview(path, maxSide ?? undefined)
      .then((data) => {
        previewResolved.set(key, data);
        return data;
      })
      .finally(() => {
        previewInflight.delete(key);
      });
    previewInflight.set(key, pending);
  }
  return pending;
}

function isImagePath(path: string) {
  const name = path.split(/[/\\]/).pop() ?? path;
  const dot = name.lastIndexOf(".");
  const ext = dot >= 0 ? name.slice(dot).toLowerCase() : "";
  return IMAGE_EXTS.has(ext);
}

export function prefetchImageThumbnails(files: FileEntry[], limit = 48) {
  for (const file of files.slice(0, limit)) {
    if (file.category === "Immagini" || isImagePath(file.source)) {
      void getPreview(file.source, THUMB_MAX_SIDE);
    }
  }
}

export function prefetchImageLightboxes(files: FileEntry[], limit = 12) {
  for (const file of files.slice(0, limit)) {
    if (file.category === "Immagini" || isImagePath(file.source)) {
      void getPreview(file.source, LIGHTBOX_MAX_SIDE);
    }
  }
}
