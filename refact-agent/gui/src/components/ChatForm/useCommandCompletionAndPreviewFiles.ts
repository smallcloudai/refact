import { useState, useEffect, useMemo, useCallback } from "react";
import { useDebounceCallback } from "usehooks-ts";
import { Checkboxes } from "./useCheckBoxes";
import { addCheckboxValuesToInput } from "./utils";
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
  // const hasCaps = useHasCaps();
  const { data } = commandsApi.useGetCommandCompletionQuery(
    { query, cursor },
    { skip: skip },
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

// TODO: this needs migrated
function useGetCommandPreviewQuery(
  _query: string,
): (ChatContextFile | string)[] {
  return [];
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
