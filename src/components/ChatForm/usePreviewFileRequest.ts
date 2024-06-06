import { useEffect, useState, useMemo } from "react";
import { type Checkbox } from "./ChatControls";

import { addCheckboxValuesToInput } from "./utils";

type UsePreviewFileRequestArgs = {
  isCommandExecutable: boolean;
  requestPreviewFiles: (input: string) => void;
  checkboxes: Record<string, Checkbox>;
  query: string;
  showControls: boolean;
  vecdb: boolean;
};
export const usePreviewFileRequest = ({
  isCommandExecutable,
  requestPreviewFiles,
  query,
  showControls,
  vecdb,
  checkboxes,
}: UsePreviewFileRequestArgs) => {
  const [prevValue, setValue] = useState<boolean>(isCommandExecutable);

  const input = useMemo(
    () => addCheckboxValuesToInput(query, checkboxes, showControls, vecdb),
    [checkboxes, query, showControls, vecdb],
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
