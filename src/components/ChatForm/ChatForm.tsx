import React, { useCallback } from "react";

import { Flex, Card, Text } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea, TextAreaProps } from "../TextArea";
import { Form } from "./Form";
import { useOnPressedEnter, useIsOnline } from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { Button } from "@radix-ui/themes";
import { ComboBox, type ComboBoxProps } from "../ComboBox";
import {
  ChatContextFile,
  CodeChatModel,
  SystemPrompts,
} from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { usePreviewFileRequest } from "./usePreviewFileRequest";
import { useAppDispatch, useAppSelector, useConfig } from "../../app/hooks";
import type { Snippet } from "../../features/Chat/selectedSnippet";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useCheckboxes } from "./useCheckBoxes";

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;

  caps: {
    error: string | null;
    fetching: boolean;
    default_cap: string;
    available_caps: Record<string, CodeChatModel>;
  };
  model: string;
  onSetChatModel: (model: string) => void;
  isStreaming: boolean;
  onStopStreaming: () => void;
  // TODO this can moved lower
  commands: ComboBoxProps["commands"];
  requestCommandsCompletion: ComboBoxProps["requestCommandsCompletion"];
  requestPreviewFiles: (input: string) => void;

  filesInPreview: ChatContextFile[];
  selectedSnippet: Snippet;

  onTextAreaHeightChange: TextAreaProps["onTextAreaHeightChange"];
  showControls: boolean;

  prompts: SystemPrompts;
  onSetSystemPrompt: (prompt: SystemPrompts) => void;
  selectedSystemPrompt: SystemPrompts;
  chatId: string;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
  caps,
  model,
  onSetChatModel,
  isStreaming,
  onStopStreaming,
  commands,
  requestCommandsCompletion,
  requestPreviewFiles,
  filesInPreview,
  onTextAreaHeightChange,
  showControls,
  prompts,
  onSetSystemPrompt,
  selectedSystemPrompt,
}) => {
  const dispatch = useAppDispatch();
  const config = useConfig();
  const error = useAppSelector(getErrorMessage);
  const onClearError = useCallback(() => dispatch(clearError()), [dispatch]);
  const [value, setValue] = React.useState("");

  const [_interacted, setInteracted] = React.useState(false);
  const [checkboxes, toggleCheckbox] = useCheckboxes();

  const refs = useTourRefs();

  usePreviewFileRequest({
    isCommandExecutable: commands.is_cmd_executable,
    requestPreviewFiles: requestPreviewFiles,
    query: value,
    vecdb: config.features?.vecdb ?? false,
    checkboxes,
  });

  const isOnline = useIsOnline();

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming && isOnline) {
      const valueIncludingChecks = addCheckboxValuesToInput(
        trimmedValue,
        checkboxes,
        config.features?.vecdb ?? false,
      );
      onSubmit(valueIncludingChecks);
      setValue(() => "");
    }
  }, [
    value,
    isStreaming,
    isOnline,
    checkboxes,
    config.features?.vecdb,
    onSubmit,
  ]);

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleChange = useCallback(
    (command: string) => {
      setInteracted(true);
      setValue(command);
    },
    [setInteracted],
  );

  if (error) {
    return (
      <ErrorCallout mt="2" onClick={onClearError} timeout={null}>
        {error}
        <Text size="1" as="div">
          Click to retry
        </Text>
      </ErrorCallout>
    );
  }

  return (
    <Card mt="1" style={{ flexShrink: 0, position: "static" }}>
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

      <Flex
        ref={(x) => refs.setChat(x)}
        style={{
          flexDirection: "column",
          alignSelf: "stretch",
          flex: 1,
          width: "100%",
        }}
      >
        <Form
          disabled={isStreaming || !isOnline}
          className={className}
          onSubmit={() => handleSubmit()}
        >
          <FilesPreview
            files={filesInPreview}
            // onRemovePreviewFile={removePreviewFileByName}
          />

          <ComboBox
            commands={commands}
            requestCommandsCompletion={requestCommandsCompletion}
            value={value}
            onChange={handleChange}
            onSubmit={(event) => {
              handleEnter(event);
            }}
            placeholder={
              commands.completions.length > 0 ? "Type @ for commands" : ""
            }
            render={(props) => (
              <TextArea
                data-testid="chat-form-textarea"
                required={true}
                disabled={isStreaming}
                {...props}
                onTextAreaHeightChange={onTextAreaHeightChange}
                autoFocus={true}
                style={{ boxShadow: "none", outline: "none" }}
              />
            )}
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
      </Flex>

      <ChatControls
        host={config.host}
        checkboxes={checkboxes}
        showControls={showControls}
        onCheckedChange={toggleCheckbox}
        selectProps={{
          value: model || caps.default_cap,
          onChange: onSetChatModel,
          options: Object.keys(caps.available_caps),
        }}
        promptsProps={{
          value: selectedSystemPrompt,
          prompts: prompts,
          onChange: onSetSystemPrompt,
        }}
      />
    </Card>
  );
};
