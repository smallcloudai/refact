import { useEffect, useRef, useCallback, useState } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import { selectLspPort } from "../features/Config/configSlice";
import {
  subscribeToChatEvents,
  type ChatEventEnvelope,
} from "../services/refact/chatSubscription";
import { applyChatEvent } from "../features/Chat/Thread/actions";

export type ConnectionStatus = "disconnected" | "connecting" | "connected";

export type UseChatSubscriptionOptions = {
  /** Enable subscription (default: true) */
  enabled?: boolean;
  /** Reconnect on error (default: true) */
  autoReconnect?: boolean;
  /** Reconnect delay in ms (default: 2000) */
  reconnectDelay?: number;
  /** Callback when event received */
  onEvent?: (event: ChatEventEnvelope) => void;
  /** Callback when connected */
  onConnected?: () => void;
  /** Callback when disconnected */
  onDisconnected?: () => void;
  /** Callback when error occurs */
  onError?: (error: Error) => void;
};

/**
 * Hook for subscribing to chat events via SSE.
 *
 * @param chatId - Chat ID to subscribe to
 * @param options - Configuration options
 * @returns Connection status and control functions
 */
export function useChatSubscription(
  chatId: string | null | undefined,
  options: UseChatSubscriptionOptions = {},
) {
  const {
    enabled = true,
    autoReconnect = true,
    reconnectDelay = 2000,
    onEvent,
    onConnected,
    onDisconnected,
    onError,
  } = options;

  const dispatch = useAppDispatch();
  const port = useAppSelector(selectLspPort);

  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [error, setError] = useState<Error | null>(null);

  const lastSeqRef = useRef<bigint>(0n);
  const callbacksRef = useRef({ onEvent, onConnected, onDisconnected, onError });
  callbacksRef.current = { onEvent, onConnected, onDisconnected, onError };

  const unsubscribeRef = useRef<(() => void) | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );

  const cleanup = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    if (unsubscribeRef.current) {
      unsubscribeRef.current();
      unsubscribeRef.current = null;
    }
  }, []);

  const connect = useCallback(() => {
    if (!chatId || !port || !enabled) return;

    cleanup();
    lastSeqRef.current = 0n;
    setStatus("connecting");
    setError(null);

    unsubscribeRef.current = subscribeToChatEvents(chatId, port, {
      onEvent: (envelope) => {
        const seq = BigInt(envelope.seq);
        if (envelope.type === "snapshot") {
          lastSeqRef.current = seq;
        } else {
          if (seq <= lastSeqRef.current) {
            return;
          }
          if (seq > lastSeqRef.current + 1n) {
            cleanup();
            setTimeout(connect, 0);
            return;
          }
          lastSeqRef.current = seq;
        }
        dispatch(applyChatEvent(envelope));
        callbacksRef.current.onEvent?.(envelope);
      },
      onConnected: () => {
        setStatus("connected");
        setError(null);
        callbacksRef.current.onConnected?.();
      },
      onDisconnected: () => {
        setStatus("disconnected");
        callbacksRef.current.onDisconnected?.();
      },
      onError: (err) => {
        setStatus("disconnected");
        setError(err);
        callbacksRef.current.onError?.(err);

        if (autoReconnect) {
          if (reconnectTimeoutRef.current) {
            clearTimeout(reconnectTimeoutRef.current);
          }
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectDelay);
        }
      },
    });
  }, [
    chatId,
    port,
    enabled,
    autoReconnect,
    reconnectDelay,
    cleanup,
    dispatch,
  ]);

  const disconnect = useCallback(() => {
    cleanup();
    setStatus("disconnected");
  }, [cleanup]);

  useEffect(() => {
    if (chatId && enabled) {
      connect();
    } else {
      disconnect();
    }

    return cleanup;
  }, [chatId, enabled, connect, disconnect, cleanup]);

  useEffect(() => {
    if (status === "connected" && chatId && enabled) {
      cleanup();
      connect();
    }
  }, [port]); // eslint-disable-line react-hooks/exhaustive-deps

  return {
    status,
    error,
    lastSeq: lastSeqRef.current.toString(),
    connect,
    disconnect,
    isConnected: status === "connected",
    isConnecting: status === "connecting",
  };
}

export default useChatSubscription;
