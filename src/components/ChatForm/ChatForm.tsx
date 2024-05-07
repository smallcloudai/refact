import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea, TextAreaProps } from "../TextArea";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  type ChatCapsState,
  useIsOnline,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { Button } from "@radix-ui/themes";
import { ComboBox, type ComboBoxProps } from "../ComboBox";
import type { ChatState } from "../../hooks";
import { ChatContextFile, SystemPrompts } from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { useConfig } from "../../contexts/config-context";
import { ChatControls, ChatControlsProps, Checkbox } from "./ChatControls";
import { useEffectOnce } from "../../hooks";
import { addCheckboxValuesToInput, activeFileToContextFile } from "./utils";

type useCheckboxStateProps = {
  activeFile: ChatState["active_file"];
  snippet: ChatState["selected_snippet"];
  vecdb: boolean;
  ast: boolean;
};

const useControlsState = ({
  activeFile,
  snippet,
  vecdb,
  ast,
}: useCheckboxStateProps) => {
  const [interacted, setInteracted] = React.useState(false);

  const fullPathWithCursor = useMemo(() => {
    if (activeFile.cursor === null) {
      return activeFile.path;
    }
    return `${activeFile.path}:${activeFile.cursor}`;
  }, [activeFile.path, activeFile.cursor]);

  const fileNameWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;
    if (!hasLines) return activeFile.name;
    return `${activeFile.name}:${activeFile.line1}-${activeFile.line2}`;
  }, [activeFile.name, activeFile.line1, activeFile.line2]);

  const filePathWithLines = useMemo(() => {
    const hasLines = activeFile.line1 !== null && activeFile.line2 !== null;

    if (!hasLines) return activeFile.path;
    return `${activeFile.path}:${activeFile.line1}-${activeFile.line2}`;
  }, [activeFile.path, activeFile.line1, activeFile.line2]);

  const markdown = useMemo(() => {
    return "```" + snippet.language + "\n" + snippet.code + "\n```\n";
  }, [snippet.language, snippet.code]);

  const codeLineCount = useMemo(() => {
    if (snippet.code.length === 0) return 0;
    return snippet.code.split("\n").length;
  }, [snippet.code]);

  const defaultState = useMemo(() => {
    return {
      search_workspace: {
        name: "search_workspace",
        checked: false,
        label: "Search workspace",
        disabled: false,
        hide: !vecdb,
        info: {
          text: "Equivalent to using @workspace. It uses the entered query to perform a search.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "blog",
        },
      },
      file_upload: {
        name: "file_upload",
        checked: !!snippet.code && !!activeFile.name,
        label: "Attach",
        value: filePathWithLines,
        disabled: !activeFile.name,
        fileName: activeFile.name,
        info: {
          text: " Similar to the @file command. It attaches the file at the current cursor position, useful for dealing with large files.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "blog",
        },
      },
      lookup_symbols: {
        name: "lookup_symbols",
        checked: !!snippet.code && !!activeFile.name,
        label: "Lookup symbols at cursor",
        value: fullPathWithCursor,
        disabled: !activeFile.name,
        hide: !ast,
        defaultChecked: !!snippet.code && !!activeFile.name,
        info: {
          text: "Corresponds to the @symbols-at command. It extracts symbols around the cursor position and searches them in the AST index.",
          link: "https://docs.refact.ai/features/ai-chat/",
          linkText: "blog",
        },
      },
      selected_lines: {
        name: "selected_lines",
        checked: !!snippet.code,
        label: `Selected ${codeLineCount} lines`,
        value: markdown,
        disabled: !snippet.code,
        info: {
          text: "Adds the currently selected lines as a snippet for analysis or modification. This is similar to embedding code within backticks ``` in the chat",
        },
      },
    } as const;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const [checkboxes, setCheckboxes] =
    React.useState<ChatControlsProps["checkboxes"]>(defaultState);

  const reset = useCallback(() => {
    setInteracted(false);
    setCheckboxes(defaultState), [defaultState];
  }, [setCheckboxes, setInteracted, defaultState]);

  const toggleCheckbox = useCallback(
    (name: string, value: boolean | string) => {
      setInteracted(true);
      setCheckboxes((prev) => {
        const checkbox: Checkbox = { ...prev[name], checked: !!value };
        const maybeAddFile: Record<string, Checkbox> =
          name === "lookup_symbols" && !!value
            ? { file_upload: { ...prev.file_upload, checked: true } }
            : { file_upload: prev.file_upload };
        const nextValue = { ...prev, ...maybeAddFile, [name]: checkbox };
        return nextValue;
      });
    },
    [setCheckboxes, setInteracted],
  );

  useEffect(() => {
    setCheckboxes((prev) => {
      const lookupDisabled = prev.lookup_symbols.checked
        ? false
        : !activeFile.name;

      const selectedLineDisabled = prev.selected_lines.checked
        ? false
        : !snippet.code;

      const fileUploadDisabled = prev.file_upload.checked
        ? false
        : !activeFile.name;

      const nextValue = {
        ...prev,
        search_workspace: {
          ...prev.search_workspace,
          hide: !vecdb,
        },
        lookup_symbols: {
          ...prev.lookup_symbols,
          value: fullPathWithCursor,
          disabled: lookupDisabled,
          hide: !ast,
          checked: interacted
            ? prev.lookup_symbols.checked
            : !!snippet.code && !!activeFile.name,
        },
        selected_lines: {
          ...prev.selected_lines,
          label: `Selected ${codeLineCount} lines`,
          value: markdown,
          disabled: selectedLineDisabled,
          checked: interacted
            ? prev.selected_lines.checked && !!snippet.code
            : !!snippet.code,
        },
        file_upload: {
          ...prev.file_upload,
          value: filePathWithLines,
          fileName: activeFile.name,
          disabled: fileUploadDisabled,
          checked: interacted
            ? prev.file_upload.checked
            : !!snippet.code && !!activeFile.name,
        },
      };

      return nextValue;
    });
  }, [
    activeFile.name,
    ast,
    codeLineCount,
    fileNameWithLines,
    filePathWithLines,
    fullPathWithCursor,
    interacted,
    markdown,
    snippet.code,
    vecdb,
  ]);

  return {
    checkboxes,
    toggleCheckbox,
    markdown,
    reset,
    setInteracted,
  };
};

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  clearError: () => void;
  error: string | null;
  caps: ChatCapsState;
  model: string;
  onSetChatModel: (model: string) => void;
  isStreaming: boolean;
  onStopStreaming: () => void;
  commands: ChatState["rag_commands"];
  attachFile: ChatState["active_file"];
  hasContextFile: boolean;
  requestCommandsCompletion: ComboBoxProps["requestCommandsCompletion"];
  requestPreviewFiles: (input: string) => void;
  setSelectedCommand: (command: string) => void;
  filesInPreview: ChatContextFile[];
  selectedSnippet: ChatState["selected_snippet"];
  removePreviewFileByName: (name: string) => void;
  onTextAreaHeightChange: TextAreaProps["onTextAreaHeightChange"];
  showControls: boolean;
  requestCaps: () => void;
  prompts: SystemPrompts;
  onSetSystemPrompt: (prompt: string) => void;
  selectedSystemPrompt: null | string;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
  error,
  clearError,
  caps,
  model,
  onSetChatModel,
  isStreaming,
  onStopStreaming,
  commands,
  attachFile,
  requestCommandsCompletion,
  requestPreviewFiles,
  setSelectedCommand,
  filesInPreview,
  selectedSnippet,
  removePreviewFileByName,
  onTextAreaHeightChange,
  showControls,
  requestCaps,
  prompts,
  onSetSystemPrompt,
  selectedSystemPrompt,
}) => {
  const config = useConfig();
  const [value, setValue] = React.useState("");
  const { markdown, checkboxes, toggleCheckbox, reset, setInteracted } =
    useControlsState({
      activeFile: attachFile,
      snippet: selectedSnippet,
      vecdb: config.features?.vecdb ?? false,
      ast: config.features?.ast ?? false,
    });

  useEffect(() => {
    if (
      caps.available_caps.length === 0 &&
      !caps.default_cap &&
      !caps.fetching
    ) {
      requestCaps();
    }
  }, [
    requestCaps,
    caps.available_caps.length,
    caps.default_cap,
    caps.fetching,
    value,
  ]);

  useEffectOnce(() => {
    if (selectedSnippet.code) {
      setValue(markdown + value);
    }
  });

  useEffect(() => {
    if (!showControls) {
      reset();
    }
  }, [showControls, reset]);

  const isOnline = useIsOnline();

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      const valueIncludingChecks = addCheckboxValuesToInput(
        trimmedValue,
        checkboxes,
        showControls,
      );
      onSubmit(valueIncludingChecks);
      setValue(() => "");
    }
  }, [value, isStreaming, isOnline, checkboxes, showControls, onSubmit]);

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleChange = useCallback(
    (command: string) => {
      setInteracted(true);
      setValue(command);
    },
    [setInteracted],
  );

  useEffect(() => {
    const input = addCheckboxValuesToInput(value, checkboxes, showControls);
    requestPreviewFiles(input);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checkboxes]);

  const handleAtCommandsRequest: ComboBoxProps["requestCommandsCompletion"] =
    useCallback(
      (
        query: string,
        cursor: number,
        trigger: string | null,
        number?: number | undefined,
      ) => {
        const inputWithCheckboxes = addCheckboxValuesToInput(
          query,
          checkboxes,
          showControls,
        );
        requestCommandsCompletion(inputWithCheckboxes, cursor, trigger, number);
      },
      [checkboxes, requestCommandsCompletion, showControls],
    );

  const previewFiles = useMemo(() => {
    const file = activeFileToContextFile(attachFile);
    if (
      showControls &&
      file.file_name &&
      checkboxes.file_upload.checked &&
      !filesInPreview.includes(file)
    ) {
      return filesInPreview.concat(file);
    }
    return filesInPreview;
  }, [
    attachFile,
    checkboxes.file_upload.checked,
    filesInPreview,
    showControls,
  ]);

  if (error) {
    return (
      <ErrorCallout mt="2" onClick={clearError} timeout={null}>
        {error}
      </ErrorCallout>
    );
  }

  return (
    <Card mt="1" style={{ position: "relative", flexShrink: 0 }}>
      {!isOnline && <Callout type="info">Offline</Callout>}

      {isStreaming && (
        <Button
          ml="auto"
          color="red"
          title="stop streaming"
          onClick={onStopStreaming}
        >
          Stop
        </Button>
      )}

      <Form
        disabled={isStreaming || !isOnline}
        className={className}
        onSubmit={() => handleSubmit()}
      >
        <FilesPreview
          files={previewFiles}
          onRemovePreviewFile={removePreviewFileByName}
        />

        <ComboBox
          commands={commands.available_commands}
          requestCommandsCompletion={handleAtCommandsRequest}
          commandArguments={commands.arguments}
          value={value}
          onChange={handleChange}
          onSubmit={(event) => {
            handleEnter(event);
          }}
          placeholder={
            commands.available_commands.length > 0 ? "Type @ for commands" : ""
          }
          render={(props) => (
            <TextArea
              data-testid="chat-form-textarea"
              required={true}
              disabled={isStreaming}
              {...props}
              onTextAreaHeightChange={onTextAreaHeightChange}
              autoFocus={true}
            />
          )}
          selectedCommand={commands.selected_command}
          setSelectedCommand={setSelectedCommand}
        />
        <Flex gap="2" className={styles.buttonGroup}>
          {onClose && (
            <BackToSideBarButton
              disabled={isStreaming}
              title="return to sidebar"
              size="1"
              onClick={onClose}
            />
          )}
          <PaperPlaneButton
            disabled={isStreaming || !isOnline}
            title="send"
            size="1"
            type="submit"
          />
        </Flex>
      </Form>

      {showControls && (
        <ChatControls
          host={config.host}
          checkboxes={checkboxes}
          onCheckedChange={toggleCheckbox}
          selectProps={{
            value: model || caps.default_cap,
            onChange: onSetChatModel,
            options: caps.available_caps,
          }}
          promptsProps={{
            value: selectedSystemPrompt ?? "",
            prompts: prompts,
            onChange: onSetSystemPrompt,
          }}
        />
      )}
    </Card>
  );
};
