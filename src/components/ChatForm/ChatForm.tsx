import React, { useCallback, useEffect, useMemo } from "react";

import { Box, Flex, Text } from "@radix-ui/themes";
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

import { Select } from "../Select/Select";
import { FileUpload } from "../FileUpload";
import { Button } from "@radix-ui/themes";
import { ComboBox, type ComboBoxProps } from "../ComboBox";
import type { ChatState } from "../../hooks";
import { ChatContextFile } from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { useConfig } from "../../contexts/config-context";
import { ChatControls, ChatControlsProps, Checkbox } from "./ChatControls";
import { useEffectOnce } from "../../hooks";

const CapsSelect: React.FC<{
  value: string;
  onChange: (value: string) => void;
  options: string[];
  disabled?: boolean;
}> = ({ options, value, onChange, disabled }) => {
  return (
    <Flex gap="2" align="center">
      <Text size="2">Use model:</Text>
      <Select
        disabled={disabled}
        title="chat model"
        options={options}
        value={value}
        onChange={onChange}
      ></Select>
    </Flex>
  );
};

type useCheckboxStateProps = {
  activeFile: ChatState["active_file"];
  snippet: ChatState["selected_snippet"];
};
const useControlsState = ({ activeFile, snippet }: useCheckboxStateProps) => {
  const lines = useMemo(() => {
    return activeFile.line1 !== null && activeFile.line2 !== null
      ? `:${activeFile.line1}-${activeFile.line2}`
      : "";
  }, [activeFile.line1, activeFile.line2]);

  const nameWithLines = useMemo(() => {
    return `${activeFile.name}${lines}`;
  }, [activeFile.name, lines]);

  const fullPathWithLines = useMemo(() => {
    return activeFile.path + lines;
  }, [activeFile.path, lines]);

  const markdown = useMemo(() => {
    return "```" + snippet.language + "\n" + snippet.code + "\n```\n";
  }, [snippet]);

  const [checkboxes, setCheckboxes] = React.useState<
    ChatControlsProps["checkboxes"]
  >({
    search_workspace: {
      name: "search_workspace",
      checked: false,
      label: "Search workspace",
      disabled: false,
    },
    lookup_symbols: {
      name: "lookup_symbols",
      checked: false,
      label: "Lookup symbols ",
      value: fullPathWithLines,
      disabled: !activeFile.name,
      fileName: nameWithLines,
    },
    selected_lines: {
      name: "selected_lines",
      checked: false,
      label: "Selected lines",
      value: markdown,
      disabled: !snippet.code,
      fileName: nameWithLines,
    },
  } as const);

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
        : fullPathWithLines;

      const lookupFileName = prev.lookup_symbols.checked
        ? prev.lookup_symbols.fileName
        : nameWithLines;

      const lookupDisabled = prev.lookup_symbols.checked
        ? false
        : !activeFile.name;

      const selectedLineValue = prev.selected_lines.checked
        ? prev.selected_lines.value
        : markdown;

      const selectedLineFileName = prev.selected_lines.checked
        ? prev.selected_lines.fileName
        : nameWithLines;

      const selectedLineDisabled = prev.selected_lines.checked
        ? false
        : !snippet.code;

      const nextValue = {
        ...prev,
        lookup_symbols: {
          ...prev.lookup_symbols,
          // TODO: should be full path,
          value: lookupValue,
          fileName: lookupFileName,
          disabled: lookupDisabled,
        },
        selected_lines: {
          ...prev.selected_lines,
          value: selectedLineValue,
          fileName: selectedLineFileName,
          disabled: selectedLineDisabled,
        },
      };

      return nextValue;
    });
  }, [
    markdown,
    nameWithLines,
    activeFile.name,
    snippet.code,
    fullPathWithLines,
  ]);

  return { checkboxes, toggleCheckbox, markdown, nameWithLines };
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
  canChangeModel: boolean;
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
  canChangeModel,
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
}) => {
  const [value, setValue] = React.useState("");
  // const [snippetAdded, setSnippetAdded] = React.useState(false);
  const { markdown, nameWithLines, checkboxes, toggleCheckbox } =
    useControlsState({
      activeFile: attachFile,
      snippet: selectedSnippet,
    });

  const addCheckboxValuesToInput = (input: string) => {
    let result = input;
    if (!result.endsWith("\n")) {
      result += "\n";
    }
    if (checkboxes.search_workspace.checked) {
      result += `@workspace\n`;
    }

    if (checkboxes.lookup_symbols.checked) {
      result += `@symbols-at ${checkboxes.lookup_symbols.value ?? ""}\n`;
    }

    if (checkboxes.selected_lines.checked) {
      result += `${checkboxes.selected_lines.value ?? ""}\n`;
    }
    return result;
  };

  const config = useConfig();

  useEffectOnce(() => {
    if (selectedSnippet.code) {
      setValue(markdown + value);
    }
  });

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

  // TODO: handle multiple files?
  const commandUpToWhiteSpace = /@file ([^\s]+)/;
  const checked = commandUpToWhiteSpace.test(value);

  return (
    <Box mt="1" position="relative">
      {!isOnline && <Callout type="info">Offline</Callout>}
      {config.host !== "web" && (
        <FileUpload
          fileName={nameWithLines}
          onClick={() =>
            setValue((preValue) => {
              if (checked) {
                return preValue.replace(commandUpToWhiteSpace, "");
              }
              const command = `@file ${nameWithLines}${
                value.length > 0 ? "\n" : ""
              }`;
              return `${command}${preValue}`;
            })
          }
          checked={checked}
          disabled={!attachFile.can_paste}
        />
      )}
      <Flex pl="2">
        {canChangeModel && (
          <CapsSelect
            value={model || caps.default_cap}
            onChange={onSetChatModel}
            options={caps.available_caps}
          />
        )}

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
      </Flex>

      <ChatControls checkboxes={checkboxes} onCheckedChange={toggleCheckbox} />

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
    </Box>
  );
};
