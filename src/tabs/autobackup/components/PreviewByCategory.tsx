import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { convertFileSrc } from "@tauri-apps/api/core";
import { FileEntry, formatBytes } from "../lib/api";
import {
  getCachedPreview,
  getPreview,
  LIGHTBOX_MAX_SIDE,
  prefetchImageThumbnails,
  prefetchImageLightboxes,
  THUMB_MAX_SIDE,
} from "../lib/previewCache";

const PREVIEW_LIMIT = 48;

const IMAGE_EXTS = new Set([
  ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".raw", ".ico", ".heic", ".heif", ".webp",
]);
const VIDEO_EXTS = new Set([".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm"]);

type PreviewMode = "pc" | "android" | null;

export type PreviewGroup = {
  category: string;
  files: FileEntry[];
  size: number;
};

function fileName(path: string) {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function fileExt(path: string) {
  const name = fileName(path);
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot).toLowerCase() : "";
}

function isImageFile(file: FileEntry) {
  return file.category === "Immagini" || IMAGE_EXTS.has(fileExt(file.source));
}

function isVideoFile(file: FileEntry) {
  return file.category === "Video" || VIDEO_EXTS.has(fileExt(file.source));
}

function assetSrc(path: string) {
  return convertFileSrc(path);
}

function canPreviewFile(mode: PreviewMode, file: FileEntry) {
  if (mode !== "pc" && mode !== "android") return false;
  return isImageFile(file) || isVideoFile(file);
}

function useMediaSrc(
  path: string,
  enabled: boolean,
  mode: PreviewMode,
  maxSide: number | null,
) {
  const [src, setSrc] = useState<string | null>(() => {
    if (!enabled) return null;
    if (mode === "pc") return assetSrc(path);
    return getCachedPreview(path, maxSide);
  });
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!enabled) {
      setSrc(null);
      setFailed(false);
      return;
    }

    if (mode === "android" || (mode === "pc" && maxSide !== null)) {
      const cached = getCachedPreview(path, maxSide);
      if (cached) {
        setSrc(cached);
        return;
      }
      let cancelled = false;
      getPreview(path, maxSide).then(
        (data) => {
          if (!cancelled) setSrc(data);
        },
        () => {
          if (!cancelled) setSrc(null);
        },
      );
      return () => {
        cancelled = true;
      };
    }

    setSrc(assetSrc(path));
    setFailed(false);
  }, [path, enabled, mode, maxSide]);

  const onError = useCallback(async () => {
    if (!enabled || failed || mode !== "pc" || maxSide !== null) return;
    setFailed(true);
    try {
      const dataUrl = await getPreview(path, THUMB_MAX_SIDE);
      setSrc(dataUrl);
    } catch {
      setSrc(null);
    }
  }, [enabled, failed, path, mode, maxSide]);

  return { src, onError };
}

function IconPdf() {
  return (
    <svg className="preview-icon preview-icon-pdf" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M14 2H7a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6M8 12h8M8 16h5" />
    </svg>
  );
}

function IconDoc() {
  return (
    <svg className="preview-icon preview-icon-doc" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M14 2H7a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6M8 12h8M8 16h8" />
    </svg>
  );
}

function IconVideo() {
  return (
    <svg className="preview-icon preview-icon-video" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="3" y="5" width="14" height="14" rx="2" />
      <path d="M17 9l4-2v10l-4-2" />
    </svg>
  );
}

function IconAudio() {
  return (
    <svg className="preview-icon preview-icon-audio" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M9 18V6l10-2v14" />
      <circle cx="7" cy="18" r="3" />
      <circle cx="17" cy="16" r="3" />
    </svg>
  );
}

function IconArchive() {
  return (
    <svg className="preview-icon preview-icon-archive" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 7h16v3H4zM4 11h16v9H4zM9 7V4h6v3" />
      <path d="M10 14h4" />
    </svg>
  );
}

function IconFile() {
  return (
    <svg className="preview-icon preview-icon-file" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M7 2h7l5 5v15a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" />
      <path d="M14 2v6h6" />
    </svg>
  );
}

function DocumentIcon({ ext }: { ext: string }) {
  if (ext === ".pdf") return <IconPdf />;
  return <IconDoc />;
}

function CategoryIcon({ category, ext }: { category: string; ext: string }) {
  switch (category) {
    case "Video":
      return <IconVideo />;
    case "Audio":
      return <IconAudio />;
    case "Archivi":
      return <IconArchive />;
    case "Documenti":
      return <DocumentIcon ext={ext} />;
    default:
      return <IconFile />;
  }
}

function PreviewLightbox({
  file,
  mode,
  onClose,
}: {
  file: FileEntry;
  mode: PreviewMode;
  onClose: () => void;
}) {
  const canPreview = canPreviewFile(mode, file);
  const isVideo = isVideoFile(file);
  const isImage = isImageFile(file);
  const lightboxMaxSide = isImage ? LIGHTBOX_MAX_SIDE : null;
  const thumbCached = isImage ? getCachedPreview(file.source, THUMB_MAX_SIDE) : null;
  const { src, onError } = useMediaSrc(file.source, canPreview, mode, lightboxMaxSide);
  const displaySrc = src ?? thumbCached;
  const isLoadingFull = isImage && canPreview && !src && !!thumbCached;
  const label = fileName(file.rel_path);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return createPortal(
    <div className="preview-lightbox" onClick={onClose} role="presentation">
      <div
        className="preview-lightbox-content"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={label}
      >
        <button type="button" className="preview-lightbox-close" onClick={onClose} aria-label="Chiudi">
          <svg className="preview-lightbox-close-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
        {canPreview && isVideo && displaySrc ? (
          <video src={displaySrc} controls autoPlay className="preview-lightbox-media" onError={onError} />
        ) : canPreview && isImage && displaySrc ? (
          <img
            src={displaySrc}
            alt={label}
            className={`preview-lightbox-media${isLoadingFull ? " is-loading" : ""}`}
            onError={onError}
          />
        ) : (
          <div className="preview-lightbox-fallback">
            <CategoryIcon category={file.category} ext={fileExt(file.source)} />
          </div>
        )}
        <p className="preview-lightbox-caption">{file.rel_path}</p>
      </div>
    </div>,
    document.body,
  );
}

function PreviewTile({
  file,
  mode,
  selected,
  onToggle,
  onOpen,
}: {
  file: FileEntry;
  mode: PreviewMode;
  selected: boolean;
  onToggle: (source: string, checked: boolean) => void;
  onOpen: (file: FileEntry) => void;
}) {
  const canPreview = canPreviewFile(mode, file);
  const ext = fileExt(file.source);
  const label = fileName(file.rel_path);
  const isImage = isImageFile(file);
  const isVideo = isVideoFile(file);
  const canOpen = canPreview;
  const thumbMaxSide = isImage ? THUMB_MAX_SIDE : null;
  const { src, onError } = useMediaSrc(file.source, canPreview, mode, thumbMaxSide);
  const showImage = isImage && !!src;
  const showVideo = isVideo && !!src;

  return (
    <div className={`preview-tile ${selected ? "is-selected" : "is-deselected"}`}>
      <label
        className="preview-check"
        onClick={(e) => e.stopPropagation()}
        title={selected ? "Escludi dal backup" : "Includi nel backup"}
      >
        <input
          type="checkbox"
          checked={selected}
          onChange={(e) => onToggle(file.source, e.target.checked)}
        />
      </label>

      <div
        role={canOpen ? "button" : undefined}
        tabIndex={canOpen ? 0 : undefined}
        className={`preview-tile-body ${showImage || showVideo ? "preview-tile-media" : "preview-tile-icon"}${canOpen ? " is-clickable" : ""}`}
        title={file.rel_path}
        onClick={() => canOpen && onOpen(file)}
        onMouseEnter={() => {
          if (canOpen && isImage && mode === "android") {
            void getPreview(file.source, LIGHTBOX_MAX_SIDE);
          }
        }}
        onKeyDown={(e) => {
          if (canOpen && (e.key === "Enter" || e.key === " ")) {
            e.preventDefault();
            onOpen(file);
          }
        }}
      >
        {showImage && (
          <img src={src} alt={label} loading="lazy" onError={onError} />
        )}
        {showVideo && (
          <>
            <video src={src} muted preload="metadata" onError={onError} />
            <span className="preview-tile-play" aria-hidden="true">▶</span>
          </>
        )}
        {!showImage && !showVideo && <CategoryIcon category={file.category} ext={ext} />}
      </div>

      <span className="preview-tile-label">{label}</span>
    </div>
  );
}

export function PreviewByCategory({
  groups,
  mode,
  selectedSources,
  onToggle,
}: {
  groups: PreviewGroup[];
  mode: PreviewMode;
  selectedSources: Set<string>;
  onToggle: (source: string, checked: boolean) => void;
}) {
  const [lightboxFile, setLightboxFile] = useState<FileEntry | null>(null);

  useEffect(() => {
    if (mode === "android") {
      const files = groups.flatMap((g) => g.files);
      prefetchImageThumbnails(files, PREVIEW_LIMIT);
      const t = window.setTimeout(() => {
        prefetchImageLightboxes(files, 12);
      }, 600);
      return () => window.clearTimeout(t);
    }
  }, [groups, mode]);

  return (
    <>
      <div className="preview-sections">
        {groups.map((group) => {
          const selectedInGroup = group.files.filter((f) => selectedSources.has(f.source)).length;
          const selectedSize = group.files
            .filter((f) => selectedSources.has(f.source))
            .reduce((sum, f) => sum + f.size, 0);

          return (
            <section key={group.category} className="preview-section">
              <h3 className="preview-section-title">
                {group.category}
                <span className="preview-section-meta">
                  {selectedInGroup}/{group.files.length} file · {formatBytes(selectedSize)}
                </span>
              </h3>
              <div className="preview-items">
                {group.files.slice(0, PREVIEW_LIMIT).map((file) => (
                  <PreviewTile
                    key={file.source}
                    file={file}
                    mode={mode}
                    selected={selectedSources.has(file.source)}
                    onToggle={onToggle}
                    onOpen={(file) =>
                      setLightboxFile((current) =>
                        current?.source === file.source ? null : file,
                      )
                    }
                  />
                ))}
              </div>
              {group.files.length > PREVIEW_LIMIT && (
                <p className="hint">
                  ...e altri {group.files.length - PREVIEW_LIMIT} file in {group.category} (selezionati per default)
                </p>
              )}
            </section>
          );
        })}
      </div>

      {lightboxFile && (
        <PreviewLightbox file={lightboxFile} mode={mode} onClose={() => setLightboxFile(null)} />
      )}
    </>
  );
}
