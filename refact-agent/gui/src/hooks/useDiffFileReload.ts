import { useEffect, useRef } from "react";
import { useAppSelector } from "./useAppSelector";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { selectMessages } from "../features/Chat/Thread/selectors";
import { selectConfig } from "../features/Config/configSlice";
import { isDiffMessage } from "../services/refact";

/**
 * Hook to handle file reloading for diff messages in JetBrains IDE
 * Ensures each file is only reloaded once per message
 */
// Note this won't work if the chat is in the cache.
export function useDiffFileReload() {
  const messages = useAppSelector(selectMessages);
  const configIdeHost = useAppSelector(selectConfig).host;
  const { setForceReloadFileByPath } = useEventsBusForIDE();

  const processedMessageIds = useRef(new Set<string>());
  const prevMessageCount = useRef(0);

  useEffect(() => {
    if (messages.length < prevMessageCount.current) {
      processedMessageIds.current.clear();
    }

    prevMessageCount.current = messages.length;

    if (messages.length === 0 || configIdeHost !== "jetbrains") {
      return;
    }

    const uniqueFilePaths = new Set<string>();

    messages.forEach((message, index) => {
      if (!isDiffMessage(message)) {
        return;
      }

      const messageId = `${message.ftm_role}-${index + 1}`;

      if (processedMessageIds.current.has(messageId)) {
        return;
      }

      processedMessageIds.current.add(messageId);

      message.ftm_content.forEach((diff) => {
        uniqueFilePaths.add(diff.file_name);
        if (diff.file_name_rename) {
          uniqueFilePaths.add(diff.file_name_rename);
        }
      });
    });

    uniqueFilePaths.forEach((filePath) => {
      setForceReloadFileByPath(filePath);
    });
  }, [messages, configIdeHost, setForceReloadFileByPath]);
}
