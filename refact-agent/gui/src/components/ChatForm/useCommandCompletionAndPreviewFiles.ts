import { useState, useEffect, useMemo, useCallback } from "react";
import { useDebounceCallback } from "usehooks-ts";
import { Checkboxes } from "./useCheckBoxes";
import {
  useAppDispatch,
  useAppSelector,
  useHasCaps,
  useSendChatRequest,
} from "../../hooks";
import { addCheckboxValuesToInput } from "./utils";
import {
  type CommandCompletionResponse,
  commandsApi,
} from "../../services/refact/commands";
import {
  ChatContextFile,
  ChatMessages,
  ChatMeta,
} from "../../services/refact/types";
import {
  selectChatId,
  selectIsStreaming,
  selectMessages,
  selectModel,
  selectThreadMode,
  setUsageTokensOnCommandPreview,
} from "../../features/Chat";

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
  const dispatch = useAppDispatch();
  const hasCaps = useHasCaps();
  const { maybeAddImagesToQuestion } = useSendChatRequest();

  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const isStreaming = useAppSelector(selectIsStreaming);
  const currentThreadMode = useAppSelector(selectThreadMode);
  const currentModel = useAppSelector(selectModel);

  const userMessage = maybeAddImagesToQuestion(query);

  const messagesToSend: ChatMessages = [...messages, userMessage];

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

  useEffect(() => {
    if (data?.current_context && data.number_context) {
      dispatch(
        setUsageTokensOnCommandPreview({
          chatId,
          n_ctx: data.number_context,
          prompt_tokens: data.current_context,
        }),
      );
    }
  }, [dispatch, chatId, data?.current_context, data?.number_context]);
  if (!data) return [];
  return data.files;
}

function useGetPreviewFiles(query: string, checkboxes: Checkboxes) {
  const queryWithCheckboxes = useMemo(
    () => addCheckboxValuesToInput(query, checkboxes),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [checkboxes, query, checkboxes.file_upload.value],
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
  }, [
    debounceSetPreviewQuery,
    queryWithCheckboxes,
    checkboxes.file_upload.value,
  ]);

  const previewFileResponse = useGetCommandPreviewQuery(previewQuery);
  return previewFileResponse;
}

export function useCommandCompletionAndPreviewFiles(checkboxes: Checkboxes) {
  const { commands, requestCompletion, query } = useCommandCompletion();
  const previewFileResponse = useGetPreviewFiles(query, checkboxes);

  return {
    commands,
    requestCompletion,
    previewFiles: previewFileResponse,
  };
}
