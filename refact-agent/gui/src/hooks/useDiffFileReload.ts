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

    if (messages.length === 0 || configIdeHost !== "jetbrains") return;
    const lastMessage = messages[messages.length - 1];

    if (!isDiffMessage(lastMessage)) return;
    const messageId = `${lastMessage.role}-${messages.length}`;

    if (processedMessageIds.current.has(messageId)) return;
    processedMessageIds.current.add(messageId);

    const uniqueFilePaths = new Set<string>();
    lastMessage.content.forEach((diff) => {
      uniqueFilePaths.add(diff.file_name);
      diff.file_name_rename && uniqueFilePaths.add(diff.file_name_rename);
    });

    uniqueFilePaths.forEach((filePath) => {
      setForceReloadFileByPath(filePath);
    });
  }, [messages, configIdeHost, setForceReloadFileByPath]);
}
