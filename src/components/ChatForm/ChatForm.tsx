import React, { useEffect } from "react";

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
  hasContextFile,
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
  const [snippetAdded, setSnippetAdded] = React.useState(false);
  const config = useConfig();

  // TODO: this won't update the value in the text area
  useEffect(() => {
    if (!snippetAdded && selectedSnippet.code) {
      setValue(
        "```" +
          selectedSnippet.language +
          "\n" +
          selectedSnippet.code +
          "\n```\n" +
          value,
      );
      setSnippetAdded(true);
    }
  }, [snippetAdded, selectedSnippet.code, value, selectedSnippet.language]);

  const isOnline = useIsOnline();

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      onSubmit(trimmedValue);
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

  const checked = value.includes(`@file ${attachFile.name}`);

  return (
    <Box mt="1" position="relative">
      {!isOnline && <Callout type="info">Offline</Callout>}
      {config.host !== "web" && !hasContextFile && (
        <FileUpload
          fileName={attachFile.name}
          onClick={() =>
            setValue((preValue) => {
              const command = `@file ${attachFile.name}${
                value.length > 0 ? "\n" : ""
              }`;
              if (checked) {
                return preValue.replace(command, "");
              }
              return `${command}${preValue}`;
            })
          }
          checked={checked}
        />
      )}
      <Flex>
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
