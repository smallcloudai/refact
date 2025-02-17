import { useState, useMemo, useCallback, useEffect } from "react";
import { selectSelectedSnippet } from "../../features/Chat/selectedSnippet";
import { selectActiveFile } from "../../features/Chat/activeFile";
import { useConfig, useAppSelector } from "../../hooks";
import type { Checkbox } from "./ChatControls";
import {
  selectIsStreaming,
  selectMessages,
} from "../../features/Chat/Thread/selectors";
import { createSelector } from "@reduxjs/toolkit";

const shouldShowSelector = createSelector(
  [selectMessages, selectIsStreaming],
  (messages, isStreaming) => {
    return messages.length === 0 && !isStreaming;
  },
);

const messageLengthSelector = createSelector(
  [selectMessages],
  (messages) => messages.length,
);

const useAttachActiveFile = (
  interacted: boolean,
  hasSnippet: boolean,
): [Checkbox, () => void] => {
  const activeFile = useAppSelector(selectActiveFile);
  const shouldShow = useAppSelector(shouldShowSelector);
  const messageLength = useAppSelector(messageLengthSelector);

  const filePathWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;

    if (!hasLines) return activeFile.path;
    return `${activeFile.path}:${
      activeFile.cursor ? activeFile.cursor + 1 : activeFile.line1
    }`;
  }, [activeFile.path, activeFile.cursor, activeFile.line1, activeFile.line2]);

  const [attachFileCheckboxData, setAttachFile] = useState<Checkbox>({
    name: "file_upload",
    checked: !!activeFile.name && messageLength === 0 && hasSnippet,
    label: "Attach",
    value: filePathWithLines,
    disabled: !activeFile.name,
    fileName: activeFile.name,
    hide: !shouldShow,
    info: {
      text: "Attaches the current file as context. If the file is large, it prefers the code near the current cursor position. Equivalent to @file name.ext:CURSOR_LINE in the text.",
      link: "https://docs.refact.ai/features/ai-chat/",
      linkText: "documentation",
    },
  });

  useEffect(() => {
    if (!interacted) {
      setAttachFile((prev) => {
        return {
          ...prev,
          hide: !shouldShow,
          value: filePathWithLines,
          disabled: !activeFile.name,
          fileName: activeFile.name,
          // checked: interacted ? prev.checked : !!activeFile.name && shouldShow,
          checked: !!activeFile.name && shouldShow && hasSnippet,
        };
      });
    }
  }, [activeFile.name, filePathWithLines, hasSnippet, interacted, shouldShow]);

  useEffect(() => {
    if (messageLength > 0 && attachFileCheckboxData.hide === false) {
      setAttachFile((prev) => {
        return { ...prev, hide: true, checked: false };
      });
    }
  }, [attachFileCheckboxData.hide, messageLength]);

  const onToggleAttachFile = useCallback(() => {
    setAttachFile((prev) => {
      return {
        ...prev,
        checked: !prev.checked,
      };
    });
  }, []);

  return [attachFileCheckboxData, onToggleAttachFile];
};

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
    if (!interacted) {
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
  }, [snippet.code, host, label, markdown, interacted]);

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
  file_upload: Checkbox;
  selected_lines: Checkbox;
};

export const useCheckboxes = () => {
  // creating 2 different states instead of only one being used for both checkboxes
  const [lineSelectionInteracted, setLineSelectionInteracted] = useState(false);
  const [fileInteracted, setFileInteracted] = useState(false);

  const [attachedSelectedSnippet, onToggleAttachedSelectedSnippet] =
    useAttachSelectedSnippet(lineSelectionInteracted);

  const [attachFileCheckboxData, onToggleAttachFile] = useAttachActiveFile(
    fileInteracted,
    attachedSelectedSnippet.checked,
  );

  const checkboxes = useMemo(
    () => ({
      file_upload: attachFileCheckboxData,
      selected_lines: attachedSelectedSnippet,
    }),
    [attachFileCheckboxData, attachedSelectedSnippet],
  );

  const onToggleCheckbox = useCallback(
    (name: string) => {
      switch (name) {
        case "file_upload":
          onToggleAttachFile();
          setFileInteracted(true);
          break;
        case "selected_lines":
          onToggleAttachedSelectedSnippet();
          setFileInteracted(true);
          setLineSelectionInteracted((prev) => !prev);
          break;
      }
    },
    [onToggleAttachFile, onToggleAttachedSelectedSnippet],
  );

  const unCheckAll = useCallback(() => {
    if (attachFileCheckboxData.checked) {
      onToggleAttachFile();
    }
    if (attachedSelectedSnippet.checked) {
      onToggleAttachedSelectedSnippet();
    }
  }, [
    attachFileCheckboxData.checked,
    attachedSelectedSnippet.checked,
    onToggleAttachFile,
    onToggleAttachedSelectedSnippet,
  ]);

  return {
    checkboxes,
    onToggleCheckbox,
    setFileInteracted,
    setLineSelectionInteracted,
    unCheckAll,
  };
};
