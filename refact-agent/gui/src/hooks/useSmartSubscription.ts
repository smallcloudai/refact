import { useEffect, useRef, useCallback, useState } from "react";
import { useSubscription } from "urql";

import { DocumentNode } from "graphql";
import { TypedDocumentNode } from "@urql/core"; // Optional, for better type safety

interface UseSmartSubscriptionArgs<
  TData = unknown,
  TVariables extends Record<string, unknown> = Record<string, unknown>,
> {
  query: string | DocumentNode | TypedDocumentNode<TData, TVariables>;
  variables?: TVariables;
  pauseAfterMs?: number;
  onUpdate?: (data: TData) => void;
  skip?: boolean;
}

interface UseSmartSubscriptionResult<TData = unknown> {
  data: TData | undefined;
  error: unknown;
  pause: () => void;
  resume: () => void;
  isSubscribed: () => boolean;
  dispose: () => void;
  refresh: () => void;
}

// Helper: useDocumentVisibility
function useDocumentVisibility() {
  const [visible, setVisible] = useState(
    document.visibilityState === "visible",
  );
  useEffect(() => {
    const handler = () => setVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", handler);
    return () => document.removeEventListener("visibilitychange", handler);
  }, []);
  return visible;
}

const THREE_MINUTES = 3 * 60 * 1000;

export function useSmartSubscription<
  TData = unknown,
  TVariables extends Record<string, unknown> = Record<string, unknown>,
>({
  query,
  variables,
  pauseAfterMs = THREE_MINUTES,
  onUpdate,
  skip = false,
}: UseSmartSubscriptionArgs<
  TData,
  TVariables
>): UseSmartSubscriptionResult<TData> {
  const [paused, setPaused] = useState(false);
  const [res, executeSubscription] = useSubscription(
    {
      query,
      variables: (variables ?? {}) as TVariables,
      pause: paused || skip,
    },
    (_, data) => {
      if (onUpdate) onUpdate(data);
      return data;
    },
  );
  const { data, error } = res;
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const visible = useDocumentVisibility();

  // Pause subscription
  const pause = useCallback(() => {
    setPaused(true);
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  // Resume subscription
  const resume = useCallback(() => {
    setPaused(false);
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  // Auto-pause after N ms if tab is hidden
  useEffect(() => {
    if (!visible) {
      timerRef.current = setTimeout(() => pause(), pauseAfterMs);
    } else {
      resume();
    }
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [visible, pauseAfterMs, pause, resume]);

  // Re-subscribe on variables change
  useEffect(() => {
    setPaused(false);
  }, [variables]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      setPaused(true);
    };
  }, []);

  return {
    data,
    error,
    pause,
    resume,
    isSubscribed: () => !paused,
    dispose: () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      setPaused(true);
    },
    refresh: () => {
      executeSubscription();
    },
  };
}
