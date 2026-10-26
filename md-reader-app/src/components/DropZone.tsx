import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

interface DropZoneProps {
  onDropPath: (path: string) => void;
  children: ReactNode;
}

const MARKDOWN_FILE = /\.(md|markdown)$/i;

/**
 * Listens to Tauri's native drag-drop events (the browser File API cannot
 * provide real file paths inside a WebView).
 */
export function DropZone({ onDropPath, children }: DropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    // Never let the WebView itself navigate to a dropped file; the native
    // drag-drop event above is the only supported path.
    const preventNavigation = (event: DragEvent) => event.preventDefault();
    window.addEventListener("dragover", preventNavigation);
    window.addEventListener("drop", preventNavigation);

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;

        if (payload.type === "enter" || payload.type === "over") {
          setIsDragging(true);
          return;
        }

        if (payload.type === "leave") {
          setIsDragging(false);
          return;
        }

        if (payload.type === "drop") {
          setIsDragging(false);
          const path =
            payload.paths.find((candidate) => MARKDOWN_FILE.test(candidate)) ??
            payload.paths[0];
          if (path) {
            onDropPath(path);
          }
        }
      })
      .then((stop) => {
        if (disposed) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((error) => {
        console.error("无法监听文件拖拽事件", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("dragover", preventNavigation);
      window.removeEventListener("drop", preventNavigation);
    };
  }, [onDropPath]);

  return (
    <div className="drop-zone">
      {children}
      {isDragging && (
        <div className="drop-overlay" aria-hidden="true">
          <div className="drop-overlay-inner">
            <span className="drop-overlay-icon">↓</span>
            <p>松开以打开 Markdown 文件</p>
          </div>
        </div>
      )}
    </div>
  );
}
