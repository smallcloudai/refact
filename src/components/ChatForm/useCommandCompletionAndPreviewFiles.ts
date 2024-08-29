import { useState, useEffect, useMemo, useCallback } from "react";
import { useDebounceCallback } from "usehooks-ts";
import { Checkboxes } from "./useCheckBoxes";
import { useAppSelector, useHasCaps } from "../../app/hooks";
import { addCheckboxValuesToInput } from "./utils";
import { selectLspPort, selectVecdb } from "../../features/Config/configSlice";
import {
  type CommandCompletionResponse,
  commandsApi,
} from "../../services/refact/commands";
import { ChatContextFile } from "../../services/refact/types";

function useGetCommandCompletionQuery(
  query: string,
  cursor: number,
  skip = false,
): CommandCompletionResponse {
  const lspPort = useAppSelector(selectLspPort);
  const hasCaps = useHasCaps();
  const { data } = commandsApi.useGetCommandCompletionQuery(
    { query, cursor, port: lspPort },
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

function useGetCommandPreviewQuery(query: string): ChatContextFile[] {
  const hasCaps = useHasCaps();
  const port = useAppSelector(selectLspPort);
  const { data } = commandsApi.useGetCommandPreviewQuery(
    { query, port },
    {
      skip: !hasCaps,
    },
  );
  if (!data) return [];
  return data;
}

function useGetPreviewFiles(
  query: string,
  checkboxes: Checkboxes,
  isExecutable: boolean,
) {
  const hasVecdb = useAppSelector(selectVecdb);
  const [wasExecutable, setWasExecutable] = useState<boolean>(isExecutable);

  const queryWithCheckboxes = useMemo(
    () => addCheckboxValuesToInput(query, checkboxes, hasVecdb),
    [checkboxes, query, hasVecdb],
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
    if (isExecutable) {
      debounceSetPreviewQuery(queryWithCheckboxes);
      setWasExecutable(true);
    } else if (wasExecutable) {
      debounceSetPreviewQuery(query);
      setWasExecutable(false);
    }
  }, [
    isExecutable,
    debounceSetPreviewQuery,
    query,
    queryWithCheckboxes,
    wasExecutable,
  ]);

  const previewFileResponse = useGetCommandPreviewQuery(previewQuery);

  return previewFileResponse;
}

export function useCommandCompletionAndPreviewFiles(checkboxes: Checkboxes) {
  const { commands, requestCompletion, query } = useCommandCompletion();

  const previewFileResponse = useGetPreviewFiles(
    query,
    checkboxes,
    commands.is_cmd_executable,
  );

  return {
    commands,
    requestCompletion,
    previewFiles: previewFileResponse,
  };
}
