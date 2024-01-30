import { useRef } from "react";

declare global {
  interface Window {
    postIntellijMessage?(message: Record<string, unknown>): void;
    acquireVsCodeApi?(): {
      postMessage: (message: Record<string, unknown>) => void;
    };
  }
}

export const usePostMessage = () => {
  const ref = useRef<typeof window.postMessage | undefined>(undefined);
  if (ref.current) return ref.current;
  if (window.acquireVsCodeApi) {
    ref.current = window.acquireVsCodeApi().postMessage;
  } else if (window.postIntellijMessage) {
    ref.current = window.postIntellijMessage.bind(this);
  } else {
    ref.current = (message: Record<string, unknown>) =>
      window.postMessage(message, "*");
  }

  return ref.current;
};
