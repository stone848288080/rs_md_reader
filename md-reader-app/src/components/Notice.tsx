import type { ReactNode } from "react";

interface NoticeProps {
  tone?: "info" | "error";
  onDismiss?: () => void;
  children: ReactNode;
}

export function Notice({ tone = "info", onDismiss, children }: NoticeProps) {
  return (
    <div
      className={`notice notice-${tone}`}
      role={tone === "error" ? "alert" : "status"}
    >
      <span className="notice-text">{children}</span>
      {onDismiss && (
        <button
          type="button"
          className="notice-close"
          aria-label="关闭提示"
          onClick={onDismiss}
        >
          ×
        </button>
      )}
    </div>
  );
}
