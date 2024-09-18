import React, { useCallback, useEffect } from "react";

import { Flex, Card, Text } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea, TextAreaProps } from "../TextArea";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  useIsOnline,
  useConfig,
  // useSendChatRequest,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { Button } from "@radix-ui/themes";
import { ComboBox } from "../ComboBox";
import {
  CodeChatModel,
  isAssistantMessage,
  SystemPrompts,
} from "../../services/refact";
import { FilesPreview } from "./FilesPreview";
import { ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { useCommandCompletionAndPreviewFiles } from "./useCommandCompletionAndPreviewFiles";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useCheckboxes } from "./useCheckBoxes";
import {
  chatGenerateTitleThunk,
  selectChatId,
  selectMessages,
  selectThreadTitle,
} from "../../features/Chat";

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
  onTextAreaHeightChange,
  showControls,
  prompts,
  onSetSystemPrompt,
  selectedSystemPrompt,
}) => {
  const dispatch = useAppDispatch();
  const config = useConfig();
  const error = useAppSelector(getErrorMessage);
  const [helpInfo, setHelpInfo] = React.useState<React.ReactNode | null>(null);
  const onClearError = useCallback(() => dispatch(clearError()), [dispatch]);
  const [value, setValue] = React.useState("");

  const { checkboxes, onToggleCheckbox, setInteracted, unCheckAll } =
    useCheckboxes();

  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const threadTitle = useAppSelector(selectThreadTitle);

  const { previewFiles, commands, requestCompletion } =
    useCommandCompletionAndPreviewFiles(checkboxes);

  const refs = useTourRefs();

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
      unCheckAll();
    }
  }, [
    value,
    isStreaming,
    isOnline,
    checkboxes,
    config.features?.vecdb,
    onSubmit,
    unCheckAll,
  ]);

  const handleEnter = useOnPressedEnter(handleSubmit);

  const handleHelpInfo = useCallback((info: React.ReactNode | null) => {
    setHelpInfo(info);
  }, []);

  const helpText = () => (
    <Flex direction="column">
      <Text size="2" weight="bold">
        Quick help for @-commands:
      </Text>
      <Text size="2">
        @definition &lt;class_or_function_name&gt; — find the definition and
        attach it.
      </Text>
      <Text size="2">
        @references &lt;class_or_function_name&gt; — find all references and
        attach them.
      </Text>
      <Text size="2">
        @file &lt;dir/filename.ext&gt; — attaches a single file to the chat.
      </Text>
      <Text size="2">@tree — workspace directory and files tree.</Text>
      <Text size="2">@web &lt;url&gt; — attach a webpage to the chat.</Text>
    </Flex>
  );

  const handleHelpCommand = useCallback(() => {
    setHelpInfo(helpText());
  }, []);

  const handleChange = useCallback(
    (command: string) => {
      setValue(command);
      setInteracted(true);
      const trimmedCommand = command.trim();
      if (trimmedCommand === "@help") {
        handleHelpInfo(helpText()); // This line has been fixed
      } else {
        handleHelpInfo(null);
      }
    },
    [setInteracted, handleHelpInfo],
  );

  useEffect(() => {
    // TODO ask for a chat title here and update the title in the chat panel
    // Also, make sure it happens only once and only when it has only one message.
    if (
      messages.filter(isAssistantMessage).find((msg) => msg.content !== "") &&
      !isStreaming
    ) {
      dispatch(chatGenerateTitleThunk({ messages, chatId }))
        .then(() => {
          return;
        })
        .catch(() => {
          return;
        });
    }
  }, [dispatch, isStreaming, chatId, messages, threadTitle]);

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

  // return <div></div>;

  return (
    <Card mt="1" style={{ flexShrink: 0, position: "static" }}>
      {!isOnline && <Callout type="info" message="Offline" />}

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
          // TODO: direction can be done with prop `direction`
          flexDirection: "column",
          alignSelf: "stretch",
          flex: 1,
          width: "100%",
        }}
      >
        {helpInfo && (
          <Flex mb="3" direction="column">
            {helpInfo}
          </Flex>
        )}
        <Form
          disabled={isStreaming || !isOnline}
          className={className}
          onSubmit={() => handleSubmit()}
        >
          <FilesPreview files={previewFiles} />

          <ComboBox
            onHelpClick={handleHelpCommand}
            commands={commands}
            requestCommandsCompletion={requestCompletion}
            value={value}
            onChange={handleChange}
            onSubmit={(event) => {
              handleEnter(event);
            }}
            placeholder={
              commands.completions.length < 1 ? "Type @ for commands" : ""
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
        onCheckedChange={onToggleCheckbox}
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
