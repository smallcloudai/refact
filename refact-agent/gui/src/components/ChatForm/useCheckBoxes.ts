import { useState, useMemo, useCallback, useEffect } from "react";
import { selectSelectedSnippet } from "../../features/Chat/selectedSnippet";
import { FileInfo, selectActiveFile } from "../../features/Chat/activeFile";
import { useConfig, useAppSelector } from "../../hooks";
import type { Checkbox } from "./ChatControls";
import { selectMessages } from "../../features/Chat/Thread/selectors";
import { createSelector } from "@reduxjs/toolkit";

const messageLengthSelector = createSelector(
  [selectMessages],
  (messages) => messages.length,
);

export function useAttachedFiles() {
  const [files, setFiles] = useState<FileInfo[]>([]);
  const activeFile = useAppSelector(selectActiveFile);

  const attached = useMemo(() => {
    const maybeAttached = files.find((file) => file.path === activeFile.path);
    return !!maybeAttached;
  }, [activeFile.path, files]);

  const addFile = useCallback(() => {
    if (attached) return;
    setFiles((prev) => {
      return [...prev, activeFile];
    });
  }, [attached, activeFile]);

  const removeFile = useCallback((fileToRemove: FileInfo) => {
    setFiles((prev) => {
      return prev.filter((file) => file.path !== fileToRemove.path);
    });
  }, []);

  const addFilesToInput = useCallback(
    (str: string) => {
      if (files.length === 0) return str;
      const result = files.reduce<string>((acc, file) => {
        const hasLines = file.line1 !== null && file.line2 !== null;
        if (!hasLines) return `@file ${file.path}\n${acc}`;
        const line = file.cursor ? file.cursor + 1 : file.line1;
        return `@file ${file.path}:${line}\n${acc}`;
      }, str);
      return result;
    },
    [files],
  );

  return {
    files,
    activeFile,
    addFile,
    removeFile,
    attached,
    addFilesToInput,
  };
}

const useAttachSelectedSnippet = (
  interacted: boolean,
): [Checkbox, () => void] => {
  const { host } = useConfig();
  const snippet = useAppSelector(selectSelectedSnippet);
  const messageLength = useAppSelector(messageLengthSelector);
  const markdown = useMemo(() => {
    return "```" + snippet.language + "\n" + snippet.code + "\n```\n";
  }, [snippet.language, snippet.code]);

  const codeLineCount = useMemo(() => {
    if (snippet.code.length === 0) return 0;
    return snippet.code.split("\n").filter((str) => str).length;
  }, [snippet.code]);

  const label = useMemo(() => {
    return `Selected ${codeLineCount} lines`;
  }, [codeLineCount]);

  const [attachedSelectedSnippet, setAttachedSelectedSnippet] =
    useState<Checkbox>({
      name: "selected_lines",
      checked: !!snippet.code && messageLength === 0,
      label: label,
      value: markdown,
      disabled: !snippet.code,
      hide: host === "web",
      info: {
        text: "Adds the currently selected lines as a snippet for analysis or modification. Equivalent to code in triple backticks ``` in the text.",
      },
    });

  useEffect(() => {
    if (!interacted || !attachedSelectedSnippet.checked) {
      setAttachedSelectedSnippet((prev) => {
        return {
          ...prev,
          label: label,
          value: markdown,
          disabled: !snippet.code,
          hide: host === "web",
          checked: !!snippet.code && !interacted,
        };
      });
    }
  }, [
    snippet.code,
    host,
    label,
    markdown,
    interacted,
    attachedSelectedSnippet.checked,
  ]);

  const onToggleAttachedSelectedSnippet = useCallback(() => {
    setAttachedSelectedSnippet((prev) => {
      return {
        ...prev,
        checked: !prev.checked,
      };
    });
  }, []);

  useEffect(() => {
    if (messageLength > 0) {
      setAttachedSelectedSnippet((prev) => {
        return {
          ...prev,
          checked: false,
        };
      });
    }
  }, [messageLength]);

  return [attachedSelectedSnippet, onToggleAttachedSelectedSnippet];
};

export type Checkboxes = {
  // file_upload: Checkbox;
  selected_lines: Checkbox;
};

export const useCheckboxes = () => {
  // creating different states instead of only one being used for checkboxes
  const [lineSelectionInteracted, setLineSelectionInteracted] = useState(false);

  const [attachedSelectedSnippet, onToggleAttachedSelectedSnippet] =
    useAttachSelectedSnippet(lineSelectionInteracted);

  const checkboxes = useMemo(
    () => ({
      selected_lines: attachedSelectedSnippet,
    }),
    [attachedSelectedSnippet],
  );

  const onToggleCheckbox = useCallback(
    (name: string) => {
      switch (name) {
        case "selected_lines":
          onToggleAttachedSelectedSnippet();
          setLineSelectionInteracted(true);
          break;
      }
    },
    [onToggleAttachedSelectedSnippet],
  );

  const unCheckAll = useCallback(() => {
    if (attachedSelectedSnippet.checked) {
      onToggleAttachedSelectedSnippet();
    }
  }, [attachedSelectedSnippet.checked, onToggleAttachedSelectedSnippet]);

  return {
    checkboxes,
    onToggleCheckbox,
    setLineSelectionInteracted,
    unCheckAll,
  };
};
