import { useState, useEffect, useMemo, useCallback } from "react";
import { useDebounceCallback } from "usehooks-ts";
import { Checkboxes } from "./useCheckBoxes";
import { useAppSelector, useHasCaps } from "../../hooks";
import { addCheckboxValuesToInput } from "./utils";
import {
  type CommandCompletionResponse,
  commandsApi,
} from "../../services/refact/commands";
import { ChatContextFile, ChatMeta, UserMessage, UserMessageContentWithImage } from "../../services/refact/types";
import type { LspChatMessage } from "../../services/refact";
import {
  getSelectedChatModel,
  selectChatId,
  selectIsStreaming,
  selectMessages,
  selectThreadMode,
  selectThreadImages,
} from "../../features/Chat";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";

function useGetCommandCompletionQuery(
  query: string,
  cursor: number,
  skip = false,
): CommandCompletionResponse {
  const hasCaps = useHasCaps();
  const { data } = commandsApi.useGetCommandCompletionQuery(
    { query, cursor },
    { skip: !hasCaps || skip },
  );

  if (!data) {
    return {
      completions: [],
      replace: [0, 0],
      is_cmd_executable: false,
    };
  }

  return data;
}

function useCommandCompletion() {
  const [command, setCommand] = useState<{
    query: string;
    cursor: number;
  } | null>(null);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const debounceSetCommand = useCallback(
    useDebounceCallback(
      (query: string, cursor: number) => setCommand({ query, cursor }),
      500,
      {
        leading: true,
        maxWait: 250,
      },
    ),
    [setCommand],
  );

  const commandCompletionResponse = useGetCommandCompletionQuery(
    command?.query ?? "",
    command?.cursor ?? 0,
    command === null,
  );

  return {
    query: command?.query ?? "",
    commands: commandCompletionResponse,
    requestCompletion: debounceSetCommand,
  };
}

function useGetCommandPreviewQuery(
  query: string,
): (ChatContextFile | string)[] {
  const hasCaps = useHasCaps();
  const attachedImages = useAppSelector(selectThreadImages);

  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const isStreaming = useAppSelector(selectIsStreaming);
  const currentThreadMode = useAppSelector(selectThreadMode);
  const currentModel = useAppSelector(getSelectedChatModel);

  const userMessage: UserMessage = useMemo(() => {
    if (attachedImages.length === 0) {
      return { role: "user", content: query, checkpoints: [] };
    }

    const images: UserMessageContentWithImage[] = attachedImages
      .filter((img) => typeof img.content === "string")
      .map((img) => ({
        type: "image_url" as const,
        image_url: { url: img.content as string },
      }));

    if (images.length === 0) {
      return { role: "user", content: query, checkpoints: [] };
    }

    return {
      role: "user",
      content: [...images, { type: "text" as const, text: query }],
      checkpoints: [],
    };
  }, [query, attachedImages]);

  const messagesToSend: LspChatMessage[] = formatMessagesForLsp([
    ...messages,
    userMessage,
  ]);

  const metaToSend: ChatMeta = {
    chat_id: chatId,
    chat_mode: currentThreadMode ?? "AGENT",
  };

  const { data } = commandsApi.useGetCommandPreviewQuery(
    { messages: messagesToSend, meta: metaToSend, model: currentModel },
    {
      skip: !hasCaps || isStreaming,
    },
  );

  if (!data) return [];
  return data.files;
}

function useGetPreviewFiles(query: string, checkboxes: Checkboxes) {
  const queryWithCheckboxes = useMemo(
    () => addCheckboxValuesToInput(query, checkboxes),
    [checkboxes, query],
  );
  const [previewQuery, setPreviewQuery] = useState<string>(queryWithCheckboxes);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const debounceSetPreviewQuery = useCallback(
    useDebounceCallback(setPreviewQuery, 500, {
      leading: true,
    }),
    [setPreviewQuery],
  );

  useEffect(() => {
    debounceSetPreviewQuery(queryWithCheckboxes);
  }, [debounceSetPreviewQuery, queryWithCheckboxes]);

  const previewFileResponse = useGetCommandPreviewQuery(previewQuery);
  return previewFileResponse;
}

export function useCommandCompletionAndPreviewFiles(
  checkboxes: Checkboxes,
  addFilesToInput: (str: string) => string,
) {
  const { commands, requestCompletion, query } = useCommandCompletion();

  const previewFileResponse = useGetPreviewFiles(
    addFilesToInput(query),
    checkboxes,
  );

  return {
    commands,
    requestCompletion,
    previewFiles: previewFileResponse,
  };
}
