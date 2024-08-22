import { useEffect, useState, useMemo } from "react";
import type { Checkboxes } from "./useCheckBoxes";

import { addCheckboxValuesToInput } from "./utils";

type UsePreviewFileRequestArgs = {
  isCommandExecutable: boolean;
  requestPreviewFiles: (input: string) => void;
  checkboxes: Checkboxes;
  query: string;
  vecdb: boolean;
};
export const usePreviewFileRequest = ({
  isCommandExecutable,
  requestPreviewFiles,
  query,
  vecdb,
  checkboxes,
}: UsePreviewFileRequestArgs) => {
  const [prevValue, setValue] = useState<boolean>(isCommandExecutable);

  const input = useMemo(
    () => addCheckboxValuesToInput(query, checkboxes, vecdb),
    [checkboxes, query, vecdb],
  );

  useEffect(() => {
    if (isCommandExecutable) {
      requestPreviewFiles(input);
      setValue(true);
    } else if (prevValue) {
      requestPreviewFiles(input);
      setValue(false);
    }
  }, [input, isCommandExecutable, prevValue, requestPreviewFiles]);

  useEffect(() => {
    requestPreviewFiles(input);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checkboxes]);
};
