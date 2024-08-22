import { useState, useMemo, useCallback, useEffect } from "react";
import { useAppSelector } from "../../app/hooks";
import { selectSelectedSnippet } from "../../features/Chat/selectedSnippet";
import { selectActiveFile } from "../../features/Chat/activeFile";
import { useConfig } from "../../app/hooks";
import type { Checkbox } from "./ChatControls";
import {
  selectIsStreaming,
  selectMessages,
} from "../../features/Chat/chatThread";
import { selectVecdb } from "../../features/Config/configSlice";
import { createSelector } from "@reduxjs/toolkit";
import { useCanUseTools } from "../../hooks/useCanUseTools";

const shouldShowSelector = createSelector(
  [selectMessages, selectIsStreaming],
  (messages, isStreaming) => {
    return messages.length === 0 && !isStreaming;
  },
);

const useAttachActiveFile = (): [Checkbox, () => void] => {
  const activeFile = useAppSelector(selectActiveFile);
  const shouldShow = useAppSelector(shouldShowSelector);

  const filePathWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;

    if (!hasLines) return activeFile.path;
    return `${activeFile.path}:${activeFile.line1}-${activeFile.line2}`;
  }, [activeFile.path, activeFile.line1, activeFile.line2]);

  const [attachFileCheckboxData, setAttachFile] = useState<Checkbox>({
    name: "file_upload",
    checked: false,
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
    if (!attachFileCheckboxData.checked) {
      setAttachFile((prev) => {
        return {
          ...prev,
          hide: !shouldShow,
          value: filePathWithLines,
          disabled: !activeFile.name,
          fileName: activeFile.name,
        };
      });
    }
  }, [
    activeFile.name,
    attachFileCheckboxData.checked,
    filePathWithLines,
    shouldShow,
  ]);

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

const useAttachSelectedSnippet = (): [Checkbox, () => void] => {
  const { host } = useConfig();
  const snippet = useAppSelector(selectSelectedSnippet);
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
      checked: !!snippet.code,
      label: label,
      value: markdown,
      disabled: !snippet.code,
      hide: host === "web",
      info: {
        text: "Adds the currently selected lines as a snippet for analysis or modification. Equivalent to code in triple backticks ``` in the text.",
      },
    });

  useEffect(() => {
    if (!attachedSelectedSnippet.checked) {
      setAttachedSelectedSnippet((prev) => {
        return {
          ...prev,
          label: label,
          value: markdown,
          disabled: !snippet.code,
          hide: host === "web",
        };
      });
    }
  }, [snippet.code, host, attachedSelectedSnippet.checked, label, markdown]);

  const onToggleAttachedSelectedSnippet = useCallback(() => {
    setAttachedSelectedSnippet((prev) => {
      return {
        ...prev,
        checked: !prev.checked,
      };
    });
  }, []);

  return [attachedSelectedSnippet, onToggleAttachedSelectedSnippet];
};

const useSearchWorkSpace = (): [Checkbox, () => void] => {
  const vecdb = useAppSelector(selectVecdb);
  const canUseTools = useCanUseTools();
  const shouldShow = useAppSelector(shouldShowSelector);

  const [searchWorkspace, setSearchWorkspace] = useState<Checkbox>({
    name: "search_workspace",
    checked: false,
    label: "Search workspace",
    disabled: false,
    hide: !vecdb || !shouldShow || canUseTools,
    info: {
      text: "Searches all files in your workspace using vector database, uses the whole text in the input box as a search query. Setting this checkbox is equivalent to @workspace command in the text.",
      link: "https://docs.refact.ai/features/ai-chat/",
      linkText: "documentation",
    },
  });

  useEffect(() => {
    setSearchWorkspace((prev) => {
      return {
        ...prev,
        hide: !vecdb || !shouldShow || canUseTools,
      };
    });
  }, [vecdb, shouldShow, canUseTools]);

  const onToggleSearchWorkspace = useCallback(() => {
    setSearchWorkspace((prev) => {
      return {
        ...prev,
        checked: !prev.checked,
      };
    });
  }, []);

  return [searchWorkspace, onToggleSearchWorkspace];
};

export type Checkboxes = {
  search_workspace: Checkbox;
  file_upload: Checkbox;
  selected_lines: Checkbox;
};

export const useCheckboxes = (): [Checkboxes, (name: string) => void] => {
  // TODO: add interacted so that auto select doesn't mess things up
  const [attachFileCheckboxData, onToggleAttachFile] = useAttachActiveFile();
  const [attachedSelectedSnippet, onToggleAttachedSelectedSnippet] =
    useAttachSelectedSnippet();
  const [searchWorkspace, onToggleSearchWorkspace] = useSearchWorkSpace();

  const checkboxes = useMemo(
    () => ({
      search_workspace: searchWorkspace,
      file_upload: attachFileCheckboxData,
      selected_lines: attachedSelectedSnippet,
    }),
    [attachFileCheckboxData, attachedSelectedSnippet, searchWorkspace],
  );

  const onToggleCheckbox = useCallback(
    (name: string) => {
      switch (name) {
        case "search_workspace":
          onToggleSearchWorkspace();
          break;
        case "file_upload":
          onToggleAttachFile();
          break;
        case "selected_lines":
          onToggleAttachedSelectedSnippet();
          break;
      }
    },
    [
      onToggleAttachFile,
      onToggleAttachedSelectedSnippet,
      onToggleSearchWorkspace,
    ],
  );

  return [checkboxes, onToggleCheckbox];
};
