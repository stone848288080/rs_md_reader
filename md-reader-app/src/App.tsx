import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

import { DropZone } from "./components/DropZone";
import { MarkdownView } from "./components/MarkdownView";
import { Notice } from "./components/Notice";
import type { OpenedFile } from "./types";

const LARGE_FILE_BYTES = 1024 * 1024;
const APP_TITLE = "Markdown Reader";

// The startup file is delivered once by the backend. Cache the request so
// React StrictMode's double effect invocation cannot consume it twice.
let initialFileRequest: Promise<OpenedFile | null> | null = null;

function loadInitialFile(): Promise<OpenedFile | null> {
  if (!initialFileRequest) {
    initialFileRequest = invoke<OpenedFile | null>("initial_file").catch(
      (error) => {
        initialFileRequest = null;
        throw error;
      },
    );
  }
  return initialFileRequest;
}

function describeError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "无法打开文件";
}

function formatSize(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function App() {
  const [file, setFile] = useState<OpenedFile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isOpening, setIsOpening] = useState(false);
  const requestId = useRef(0);

  const openPath = useCallback(async (path: string) => {
    const request = ++requestId.current;
    setIsOpening(true);
    setError(null);

    try {
      const opened = await invoke<OpenedFile>("open_file", { path });
      if (request === requestId.current) {
        setFile(opened);
      }
    } catch (cause) {
      if (request === requestId.current) {
        setError(describeError(cause));
      }
    } finally {
      if (request === requestId.current) {
        setIsOpening(false);
      }
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    const start = async () => {
      try {
        const stop = await listen<OpenedFile>("open-file", (event) => {
          if (disposed) {
            return;
          }
          requestId.current += 1;
          setFile(event.payload);
          setError(null);
          setIsOpening(false);
        });

        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;

        const initial = await loadInitialFile();
        if (!disposed && initial) {
          setFile(initial);
        }
      } catch (cause) {
        if (!disposed) {
          setError(describeError(cause));
        }
      }
    };

    void start();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    document.title = file?.name ?? APP_TITLE;
  }, [file]);

  const dismissError = () => setError(null);

  return (
    <DropZone onDropPath={openPath}>
      {file ? (
        <main className="reader">
          {error && (
            <Notice tone="error" onDismiss={dismissError}>
              {error}
            </Notice>
          )}
          {!error && file.size > LARGE_FILE_BYTES && (
            <Notice tone="info">
              文件较大（{formatSize(file.size)}），渲染可能需要一点时间。
            </Notice>
          )}
          <MarkdownView content={file.content} baseDir={file.baseDir} />
        </main>
      ) : (
        <main className="empty">
          <div className="empty-content">
            <svg
              className="empty-icon"
              aria-hidden="true"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.3"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <path d="M14 2v6h6" />
              <path d="M8 13h8" />
              <path d="M8 17h5" />
            </svg>
            <h1 className="empty-title">拖入 .md 文件开始阅读</h1>
            <p className="empty-hint">
              {isOpening ? "正在打开…" : "也可以双击已关联的 Markdown 文件"}
            </p>
            {error && (
              <Notice tone="error" onDismiss={dismissError}>
                {error}
              </Notice>
            )}
          </div>
        </main>
      )}
    </DropZone>
  );
}
