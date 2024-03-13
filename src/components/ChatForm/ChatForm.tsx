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
import { ChatContextFile } from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { useConfig } from "../../contexts/config-context";
import { ChatControls, ChatControlsProps, Checkbox } from "./ChatControls";
import { useEffectOnce } from "../../hooks";

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
  const lines = useMemo(() => {
    return activeFile.line1 !== null && activeFile.line2 !== null
      ? `:${activeFile.line1}-${activeFile.line2}`
      : "";
  }, [activeFile.line1, activeFile.line2]);

  const nameWithLines = useMemo(() => {
    return `${activeFile.name}${lines}`;
  }, [activeFile.name, lines]);

  const nameWithCursor = useMemo(() => {
    if (activeFile.cursor === null) {
      return activeFile.name;
    }
    return `${activeFile.name}:${activeFile.cursor}`;
  }, [activeFile.name, activeFile.cursor]);

  const fullPathWithLines = useMemo(() => {
    return activeFile.path + lines;
  }, [activeFile.path, lines]);

  const fullPathWithCursor = useMemo(() => {
    if (activeFile.cursor === null) {
      return activeFile.path;
    }
    return `${activeFile.path}:${activeFile.cursor}`;
  }, [activeFile.path, activeFile.cursor]);

  const markdown = useMemo(() => {
    return "```" + snippet.language + "\n" + snippet.code + "\n```\n";
  }, [snippet.language, snippet.code]);

  const defaultState = useMemo(() => {
    return {
      search_workspace: {
        name: "search_workspace",
        checked: false,
        label: "Search workspace",
        disabled: false,
        hide: !vecdb,
      },
      file_upload: {
        name: "file_upload",
        checked: false,
        label: "Attach",
        value: fullPathWithLines,
        disabled: !activeFile.name,
        fileName: nameWithCursor,
      },
      lookup_symbols: {
        name: "lookup_symbols",
        checked: false,
        label: "Lookup symbols at cursor",
        value: fullPathWithCursor,
        disabled: !activeFile.name,
        hide: !ast,
        // fileName: " at cursor",
        // fileName: nameWithCursor,
      },
      selected_lines: {
        name: "selected_lines",
        checked: false,
        label: "Selected N lines",
        value: markdown,
        disabled: !snippet.code,
        // fileName: nameWithLines,
      },
    } as const;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const [checkboxes, setCheckboxes] =
    React.useState<ChatControlsProps["checkboxes"]>(defaultState);

  const reset = useCallback(() => setCheckboxes(defaultState), [defaultState]);

  const toggleCheckbox = useCallback(
    (name: string, value: boolean | string) => {
      setCheckboxes((prev) => {
        const checkbox: Checkbox = { ...prev[name], checked: !!value };
        const nextValue = { ...prev, [name]: checkbox };
        return nextValue;
      });
    },
    [setCheckboxes],
  );

  useEffect(() => {
    setCheckboxes((prev) => {
      const lookupValue = prev.lookup_symbols.checked
        ? prev.lookup_symbols.value
        : fullPathWithCursor;

      // const lookupFileName = prev.lookup_symbols.checked
      //   ? prev.lookup_symbols.fileName
      //   : nameWithCursor;

      const lookupDisabled = prev.lookup_symbols.checked
        ? false
        : !activeFile.name;

      // const selectedLineValue = prev.selected_lines.checked
      //   ? prev.selected_lines.value
      //   : markdown;

      // const selectedLineFileName = prev.selected_lines.checked
      //   ? prev.selected_lines.fileName
      //   : nameWithLines;

      const selectedLineDisabled = prev.selected_lines.checked
        ? false
        : !snippet.code;

      const fileUploadValue = prev.file_upload.checked
        ? prev.file_upload.value
        : fullPathWithCursor;

      const fileUploadFileName = prev.file_upload.checked
        ? prev.file_upload.fileName
        : activeFile.name;

      const fileUploadDisabled = prev.file_upload.checked
        ? false
        : !activeFile.name;

      const nextValue = {
        ...prev,
        search_workspace: {
          ...prev.search_workspace,
          hide: vecdb,
        },
        lookup_symbols: {
          ...prev.lookup_symbols,
          value: lookupValue,
          // fileName: lookupFileName,
          disabled: lookupDisabled,
          hide: !ast,
        },
        selected_lines: {
          ...prev.selected_lines,
          // maybe allow this to change?
          // value: selectedLineValue,
          value: markdown,
          // fileName: selectedLineFileName,
          disabled: selectedLineDisabled,
        },
        file_upload: {
          ...prev.file_upload,
          value: fileUploadValue,
          fileName: fileUploadFileName,
          disabled: fileUploadDisabled,
        },
      };

      return nextValue;
    });
  }, [
    markdown,
    nameWithLines,
    nameWithCursor,
    activeFile.name,
    snippet.code,
    fullPathWithLines,
    fullPathWithCursor,
    vecdb,
    ast,
  ]);

  return {
    checkboxes,
    toggleCheckbox,
    markdown,
    nameWithLines,
    fullPathWithLines,
    reset,
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
  setSelectedCommand: (command: string) => void;
  filesInPreview: ChatContextFile[];
  selectedSnippet: ChatState["selected_snippet"];
  removePreviewFileByName: (name: string) => void;
  onTextAreaHeightChange: TextAreaProps["onTextAreaHeightChange"];
  showControls: boolean;
  requestCaps: () => void;
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
  setSelectedCommand,
  filesInPreview,
  selectedSnippet,
  removePreviewFileByName,
  onTextAreaHeightChange,
  showControls,
  requestCaps,
}) => {
  const config = useConfig();
  const [value, setValue] = React.useState("");
  const { markdown, checkboxes, toggleCheckbox, reset } = useControlsState({
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

  const addCheckboxValuesToInput = (input: string) => {
    if (!showControls) {
      return input;
    }

    let result = input;
    if (!result.endsWith("\n")) {
      result += "\n";
    }
    if (
      checkboxes.search_workspace.checked &&
      checkboxes.search_workspace.hide !== true
    ) {
      result += `@workspace\n`;
    }

    if (
      checkboxes.lookup_symbols.checked &&
      checkboxes.lookup_symbols.hide !== true
    ) {
      result += `@symbols-at ${checkboxes.lookup_symbols.value ?? ""}\n`;
    }

    if (
      checkboxes.selected_lines.checked &&
      checkboxes.selected_lines.hide !== true
    ) {
      result += `${checkboxes.selected_lines.value ?? ""}\n`;
    }

    if (
      checkboxes.file_upload.checked &&
      checkboxes.file_upload.hide !== true
    ) {
      result += `@file ${checkboxes.file_upload.value ?? ""}\n`;
    }

    return result;
  };

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

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      const valueIncludingChecks = addCheckboxValuesToInput(trimmedValue);
      onSubmit(valueIncludingChecks);
      setValue(() => "");
    }
  };

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleChange = (command: string) => {
    setValue(command);
  };
  if (error) {
    return (
      <ErrorCallout mt="2" onClick={clearError} timeout={null}>
        {error}
      </ErrorCallout>
    );
  }

  return (
    <Card mt="1" style={{ position: "relative" }}>
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
        />
      )}

      {/** TODO: handle being offline */}

      <Form
        disabled={isStreaming || !isOnline}
        className={className}
        onSubmit={() => handleSubmit()}
      >
        <FilesPreview
          files={filesInPreview}
          onRemovePreviewFile={removePreviewFileByName}
        />

        <ComboBox
          commands={commands.available_commands}
          requestCommandsCompletion={requestCommandsCompletion}
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
    </Card>
  );
};
