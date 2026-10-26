import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { renderMarkdown } from "../lib/markdown";

interface MarkdownViewProps {
  content: string;
  baseDir: string;
}

export function MarkdownView({ content, baseDir }: MarkdownViewProps) {
  const html = useMemo(
    () => renderMarkdown(content, baseDir),
    [content, baseDir],
  );
  const containerRef = useRef<HTMLElement | null>(null);

  // Opening another document should always start from the top.
  useLayoutEffect(() => {
    window.scrollTo(0, 0);
  }, [html]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const handleClick = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }

      const anchor = target.closest("a");
      if (!(anchor instanceof HTMLAnchorElement)) {
        return;
      }

      // Never navigate inside the WebView. External links go to the system
      // browser; in-document anchors scroll the reading view.
      event.preventDefault();
      const href = anchor.getAttribute("href")?.trim();
      if (!href) {
        return;
      }

      if (href.startsWith("#")) {
        const destination = findAnchor(href.slice(1));
        destination?.scrollIntoView({ behavior: "smooth", block: "start" });
        return;
      }

      if (/^(https?:|mailto:|tel:)/i.test(href)) {
        void openUrl(href).catch((error) => {
          console.error("无法用系统浏览器打开链接", error);
        });
      }
    };

    const handleImageError = (event: Event) => {
      if (event.target instanceof HTMLImageElement) {
        replaceBrokenImage(event.target);
      }
    };

    container.addEventListener("click", handleClick);
    container.addEventListener("error", handleImageError, true);

    return () => {
      container.removeEventListener("click", handleClick);
      container.removeEventListener("error", handleImageError, true);
    };
  }, [html]);

  return (
    <article
      ref={containerRef}
      className="markdown-body"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

function findAnchor(rawId: string): HTMLElement | null {
  if (!rawId) {
    return null;
  }

  let id = rawId;
  try {
    id = decodeURIComponent(rawId);
  } catch {
    // Keep the raw fragment when it is not valid percent-encoding.
  }

  const byId = document.getElementById(id);
  if (byId) {
    return byId;
  }

  try {
    return document.querySelector<HTMLElement>(`[name="${CSS.escape(id)}"]`);
  } catch {
    return null;
  }
}

function replaceBrokenImage(image: HTMLImageElement) {
  if (image.dataset.broken === "true") {
    return;
  }
  image.dataset.broken = "true";

  const placeholder = document.createElement("div");
  placeholder.className = "image-error";
  const alt = image.alt.trim();
  placeholder.textContent = alt ? `图片无法加载：${alt}` : "图片无法加载";
  image.replaceWith(placeholder);
}
