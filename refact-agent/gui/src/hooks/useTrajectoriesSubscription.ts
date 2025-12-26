import { useEffect, useRef, useCallback } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useConfig } from "./useConfig";
import {
  trajectoriesApi,
  TrajectoryEvent,
  chatThreadToTrajectoryData,
  trajectoryDataToChatThread,
} from "../services/refact/trajectories";
import { hydrateHistory, deleteChatById, ChatHistoryItem } from "../features/History/historySlice";
import { updateOpenThread, closeThread } from "../features/Chat/Thread";

const MIGRATION_KEY = "refact-trajectories-migrated";

function getLegacyHistory(): ChatHistoryItem[] {
  try {
    const raw = localStorage.getItem("persist:root");
    if (!raw) return [];

    const parsed = JSON.parse(raw) as Record<string, string>;
    if (!parsed.history) return [];

    const historyState = JSON.parse(parsed.history) as Record<string, ChatHistoryItem>;
    return Object.values(historyState);
  } catch {
    return [];
  }
}

function clearLegacyHistory() {
  try {
    const raw = localStorage.getItem("persist:root");
    if (!raw) return;

    const parsed = JSON.parse(raw) as Record<string, string>;
    parsed.history = "{}";
    localStorage.setItem("persist:root", JSON.stringify(parsed));
  } catch {
    // ignore
  }
}

function isMigrationDone(): boolean {
  return localStorage.getItem(MIGRATION_KEY) === "true";
}

function markMigrationDone() {
  localStorage.setItem(MIGRATION_KEY, "true");
}

export function useTrajectoriesSubscription() {
  const dispatch = useAppDispatch();
  const config = useConfig();
  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (typeof EventSource === "undefined") return;

    const port = config.lspPort;
    const url = `http://127.0.0.1:${port}/v1/trajectories/subscribe`;

    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    try {
      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;

      eventSource.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data as string) as TrajectoryEvent;
          if (data.type === "deleted") {
            dispatch(deleteChatById(data.id));
            // Force delete runtime even if it's streaming - backend says it's gone
            dispatch(closeThread({ id: data.id, force: true }));
          } else {
            void dispatch(
              trajectoriesApi.endpoints.getTrajectory.initiate(data.id, {
                forceRefetch: true,
              }),
            )
              .unwrap()
              .then((trajectory) => {
                dispatch(hydrateHistory([trajectory]));
                const thread = trajectoryDataToChatThread(trajectory);
                dispatch(updateOpenThread({
                  id: data.id,
                  thread: {
                    title: thread.title,
                    isTitleGenerated: thread.isTitleGenerated,
                  },
                }));
              })
              .catch(() => undefined);
          }
        } catch {
          // ignore parse errors
        }
      };

      eventSource.onerror = () => {
        eventSource.close();
        // Clear any existing reconnect timer before scheduling a new one
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
        }
        reconnectTimeoutRef.current = setTimeout(connect, 5000);
      };
    } catch {
      // EventSource not available or connection failed
    }
  }, [dispatch, config.lspPort]);

  const migrateFromLocalStorage = useCallback(async () => {
    if (isMigrationDone()) return;

    const legacyChats = getLegacyHistory();
    if (legacyChats.length === 0) {
      markMigrationDone();
      return;
    }

    let successCount = 0;
    for (const chat of legacyChats) {
      if (chat.messages.length === 0) continue;

      try {
        const trajectoryData = chatThreadToTrajectoryData(
          {
            ...chat,
            new_chat_suggested: chat.new_chat_suggested ?? { wasSuggested: false },
          },
          chat.createdAt,
        );
        trajectoryData.updated_at = chat.updatedAt;

        await dispatch(
          trajectoriesApi.endpoints.saveTrajectory.initiate(trajectoryData),
        ).unwrap();
        successCount++;
      } catch {
        // Failed to migrate this chat, continue with others
      }
    }

    if (successCount > 0) {
      clearLegacyHistory();
    }
    markMigrationDone();
  }, [dispatch]);

  const loadInitialHistory = useCallback(async () => {
    try {
      await migrateFromLocalStorage();

      const result = await dispatch(
        trajectoriesApi.endpoints.listTrajectories.initiate(undefined),
      ).unwrap();

      const trajectories = await Promise.all(
        result.map((meta) =>
          dispatch(
            trajectoriesApi.endpoints.getTrajectory.initiate(meta.id),
          ).unwrap(),
        ),
      );

      dispatch(hydrateHistory(trajectories));
    } catch {
      // Backend not available
    }
  }, [dispatch, migrateFromLocalStorage]);

  useEffect(() => {
    void loadInitialHistory();
    connect();

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, [connect, loadInitialHistory]);
}
