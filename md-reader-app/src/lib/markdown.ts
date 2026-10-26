import { convertFileSrc } from "@tauri-apps/api/core";
import MarkdownIt from "markdown-it";

interface RenderEnv {
  baseDir: string;
}

const markdown = new MarkdownIt({
  // Never render raw HTML from the document: the reader must not execute
  // scripts embedded in Markdown files.
  html: false,
  linkify: true,
  breaks: false,
  typographer: false,
});

const defaultImageRenderer = markdown.renderer.rules.image;

markdown.renderer.rules.image = (tokens, idx, options, env, self) => {
  const token = tokens[idx];
  const src = token.attrGet("src");
  if (typeof src === "string" && src) {
    const { baseDir } = env as unknown as Partial<RenderEnv>;
    token.attrSet("src", resolveImageSource(src, baseDir ?? ""));
  }
  token.attrSet("loading", "lazy");
  token.attrSet("decoding", "async");
  return defaultImageRenderer?.(tokens, idx, options, env, self) ?? self.renderToken(tokens, idx, options);
};

// Tables need a wrapper so wide tables scroll horizontally instead of
// stretching the reading column.
markdown.renderer.rules.table_open = () => '<div class="table-wrap"><table>\n';
markdown.renderer.rules.table_close = () => "</table></div>\n";

markdown.renderer.rules.link_open = (tokens, idx, options, _env, self) => {
  tokens[idx].attrSet("rel", "noopener noreferrer");
  return self.renderToken(tokens, idx, options);
};

export function renderMarkdown(content: string, baseDir: string): string {
  return markdown.render(content, { baseDir } satisfies RenderEnv);
}

/**
 * Turn a local image reference into an asset-protocol URL that the WebView is
 * allowed to load. Remote images and data URLs are left untouched.
 */
export function resolveImageSource(source: string, baseDir: string): string {
  const value = source.trim();
  if (!value || value.startsWith("#")) {
    return value;
  }

  if (isWindowsAbsolutePath(value)) {
    return convertFileSrc(normalizeLocalPath(value));
  }

  if (hasUrlScheme(value)) {
    if (value.toLowerCase().startsWith("file:")) {
      const localPath = pathFromFileUrl(value);
      return localPath ? convertFileSrc(normalizeLocalPath(localPath)) : value;
    }
    return value;
  }

  // Strip an anchor/query before treating the value as a file path. URL-encoded
  // characters (e.g. `%20`) are decoded back to real path characters.
  const withoutSuffix = value.split("#")[0].split("?")[0];
  const decoded = decodePath(withoutSuffix);
  const absolute = decoded.startsWith("/") || isWindowsAbsolutePath(decoded)
    ? decoded
    : joinLocalPath(baseDir, decoded);

  return convertFileSrc(normalizeLocalPath(absolute));
}

const WINDOWS_ABSOLUTE_PATH = /^[a-zA-Z]:[\\/]/;

function isWindowsAbsolutePath(value: string): boolean {
  return WINDOWS_ABSOLUTE_PATH.test(value) || value.startsWith("\\\\");
}

function hasUrlScheme(value: string): boolean {
  return /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(value);
}

function decodePath(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function pathFromFileUrl(href: string): string | null {
  try {
    const url = new URL(href);
    let pathname = decodeURIComponent(url.pathname);
    // file:///C:/foo -> C:/foo
    if (/^\/[a-zA-Z]:\//.test(pathname)) {
      pathname = pathname.slice(1);
    }
    if (url.hostname && url.hostname !== "localhost") {
      pathname = `//${url.hostname}${pathname}`;
    }
    return pathname;
  } catch {
    return null;
  }
}

function joinLocalPath(baseDir: string, relativePath: string): string {
  const separator = isWindowsAbsolutePath(baseDir) ? "\\" : "/";
  const base = baseDir.replace(/[\\/]+$/, "");
  return `${base}${separator}${relativePath}`;
}

function normalizeLocalPath(input: string): string {
  if (isWindowsAbsolutePath(input)) {
    const driveMatch = /^([a-zA-Z]:)/.exec(input);
    const drive = driveMatch?.[1] ?? "";
    const isUnc = input.startsWith("\\\\");
    const rest = drive ? input.slice(drive.length) : input;
    const segments = collapseSegments(rest.split(/[\\/]+/));
    return isUnc
      ? `\\\\${segments.join("\\")}`
      : `${drive}\\${segments.join("\\")}`;
  }

  const isAbsolute = input.startsWith("/");
  const segments = collapseSegments(input.split("/"));
  return `${isAbsolute ? "/" : ""}${segments.join("/")}`;
}

function collapseSegments(parts: string[]): string[] {
  const stack: string[] = [];
  for (const part of parts) {
    if (!part || part === ".") {
      continue;
    }
    if (part === "..") {
      stack.pop();
      continue;
    }
    stack.push(part);
  }
  return stack;
}
