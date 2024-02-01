import React from "react";

import { Box, Flex, Text } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea } from "../TextArea";
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
import { ComboBox } from "../ComboBox";
import type { ChatState } from "../../hooks";

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

export const ChatForm: React.FC<{
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
  handleContextFile: () => void;
  hasContextFile: boolean;
  commands: ChatState["rag_commands"];
  attachFile: ChatState["active_file"];
  setSelectedCommand: (command: string) => void;
  requestCommandsCompletion: (
    query: string,
    cursor: number,
    number?: number,
  ) => void;
}> = ({
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
  handleContextFile,
  hasContextFile,
  commands,
  attachFile,
  requestCommandsCompletion,
  setSelectedCommand,
}) => {
  const [value, setValue] = React.useState("");

  const isOnline = useIsOnline();

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      onSubmit(trimmedValue);
      setValue(() => "");
    }
  };

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleChange = (command: React.SetStateAction<string>) => {
    setValue(command);

    const str = typeof command === "function" ? command(value) : command;

    if (
      str.endsWith(" ") &&
      commands.available_commands.includes(str.slice(0, -1))
    ) {
      setSelectedCommand(str);
    } else {
      setSelectedCommand("");
    }

    // TODO: get the  cursor position
    // Does order matter here?
    if (str.startsWith("@")) {
      requestCommandsCompletion(str, str.length);
    }
    // set selected command if value ends with space and is in commands
  };
  if (error) {
    return (
      <ErrorCallout mt="2" onClick={clearError} timeout={null}>
        {error}
      </ErrorCallout>
    );
  }

  // console.log({ commands });

  return (
    <Box mt="1" position="relative">
      {!isOnline && <Callout type="info">Offline</Callout>}
      {canChangeModel && (
        <FileUpload
          fileName={attachFile.name}
          onClick={handleContextFile}
          checked={hasContextFile || attachFile.attach}
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
        <ComboBox
          // maybe add a ref for cursor position?
          commands={commands.available_commands.map(
            (c) => commands.selected_command + c,
          )}
          value={value}
          onChange={handleChange}
          onSubmit={(event) => {
            // console.log("submit", event);
            handleEnter(event);
          }}
          placeholder={
            commands.available_commands.length > 0 ? "Type @ for commands" : ""
          }
          render={(props) => <TextArea disabled={isStreaming} {...props} />}
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
