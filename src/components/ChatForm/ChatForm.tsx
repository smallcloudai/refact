import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card, Text } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  useIsOnline,
  useConfig,
  useAgentUsage,
  useSendChatRequest,
  useCapsForToolUse,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { ComboBox } from "../ComboBox";
import { FilesPreview } from "./FilesPreview";
import { ApplyPatchSwitch, ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { useCommandCompletionAndPreviewFiles } from "./useCommandCompletionAndPreviewFiles";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useCheckboxes } from "./useCheckBoxes";
import { useInputValue } from "./useInputValue";
import {
  clearInformation,
  getInformationMessage,
  setInformation,
} from "../../features/Errors/informationSlice";
import { InformationCallout } from "../Callout/Callout";
import { ToolConfirmation } from "./ToolConfirmation";
import { getPauseReasonsWithPauseStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { AttachFileButton, FileList } from "../Dropzone";
import { useAttachedImages } from "../../hooks/useAttachedImages";
import {
  enableSend,
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectPreventSend,
  selectToolUse,
} from "../../features/Chat";
import { isUserMessage } from "../../services/refact";

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  unCalledTools: boolean;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
  unCalledTools,
}) => {
  const dispatch = useAppDispatch();
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const { retryFromIndex } = useSendChatRequest();
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
  const config = useConfig();
  const toolUse = useAppSelector(selectToolUse);
  const error = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);
  const pauseReasonsWithPause = useAppSelector(getPauseReasonsWithPauseStatus);
  const [helpInfo, setHelpInfo] = React.useState<React.ReactNode | null>(null);
  const { disableInput } = useAgentUsage();
  const isOnline = useIsOnline();

  const chatId = useAppSelector(selectChatId);
  const messages = useAppSelector(selectMessages);
  const preventSend = useAppSelector(selectPreventSend);

  const onClearError = useCallback(() => {
    dispatch(clearError());
    const userMessages = messages.filter(isUserMessage);

    // getting second-to-last user message
    const lastSuccessfulUserMessage =
      userMessages.slice(-2, -1)[0] || userMessages[0];

    const lastSuccessfulUserMessageIndex = messages.indexOf(
      lastSuccessfulUserMessage,
    );
    retryFromIndex(
      lastSuccessfulUserMessageIndex,
      lastSuccessfulUserMessage.content,
    );
  }, [dispatch, retryFromIndex, messages]);

  const disableSend = useMemo(() => {
    // TODO: if interrupting chat some errors can occur
    if (messages.length === 0) return false;
    return isWaiting || isStreaming || !isOnline || preventSend;
  }, [isOnline, isStreaming, isWaiting, preventSend, messages]);

  const { processAndInsertImages } = useAttachedImages();
  const handlePastingFile = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      if (!isMultimodalitySupportedForCurrentModel) return;
      const files: File[] = [];
      const items = event.clipboardData.items;
      for (const item of items) {
        if (item.kind === "file") {
          const file = item.getAsFile();
          file && files.push(file);
        }
      }
      if (files.length > 0) {
        event.preventDefault();
        processAndInsertImages(files);
      }
    },
    [processAndInsertImages, isMultimodalitySupportedForCurrentModel],
  );

  const {
    checkboxes,
    onToggleCheckbox,
    unCheckAll,
    setFileInteracted,
    setLineSelectionInteracted,
  } = useCheckboxes();

  const [value, setValue, isSendImmediately, setIsSendImmediately] =
    useInputValue(() => unCheckAll());

  const onClearInformation = useCallback(
    () => dispatch(clearInformation()),
    [dispatch],
  );

  const { previewFiles, commands, requestCompletion } =
    useCommandCompletionAndPreviewFiles(checkboxes);

  const refs = useTourRefs();

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    if (disableInput) {
      const action = setInformation(
        "You have exceeded the FREE usage limit, upgrade to PRO or switch to EXPLORE mode.",
      );
      dispatch(action);
    } else if (!disableSend && trimmedValue.length > 0) {
      const valueIncludingChecks = addCheckboxValuesToInput(
        trimmedValue,
        checkboxes,
        config.features?.vecdb ?? false,
      );
      setFileInteracted(false);
      setLineSelectionInteracted(false);
      onSubmit(valueIncludingChecks);
      setValue(() => "");
      unCheckAll();
    }
  }, [
    value,
    disableInput,
    disableSend,
    dispatch,
    checkboxes,
    config.features?.vecdb,
    setFileInteracted,
    setLineSelectionInteracted,
    onSubmit,
    setValue,
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
      const trimmedCommand = command.trim();
      setFileInteracted(!!trimmedCommand);
      setLineSelectionInteracted(!!trimmedCommand);
      if (trimmedCommand === "@help") {
        handleHelpInfo(helpText()); // This line has been fixed
      } else {
        handleHelpInfo(null);
      }
    },
    [handleHelpInfo, setValue, setFileInteracted, setLineSelectionInteracted],
  );

  useEffect(() => {
    // this use effect is required to reset preventSend when chat was restored
    if (
      preventSend &&
      !unCalledTools &&
      !isStreaming &&
      !isWaiting &&
      isOnline
    ) {
      dispatch(enableSend({ id: chatId }));
    }
  }, [
    dispatch,
    isOnline,
    isWaiting,
    isStreaming,
    preventSend,
    chatId,
    unCalledTools,
  ]);

  useEffect(() => {
    if (isSendImmediately && !isWaiting && !isStreaming) {
      handleSubmit();
      setIsSendImmediately(false);
    }
  }, [
    isSendImmediately,
    isWaiting,
    isStreaming,
    handleSubmit,
    setIsSendImmediately,
  ]);

  if (error) {
    return (
      <ErrorCallout mt="2" onClick={onClearError} timeout={null}>
        {error}
      </ErrorCallout>
    );
  }

  if (information) {
    return (
      <InformationCallout mt="2" onClick={onClearInformation} timeout={2000}>
        {information}
      </InformationCallout>
    );
  }

  if (!isStreaming && pauseReasonsWithPause.pause) {
    return (
      <ToolConfirmation pauseReasons={pauseReasonsWithPause.pauseReasons} />
    );
  }

  return (
    <Card mt="1" style={{ flexShrink: 0, position: "static" }}>
      {!isOnline && <Callout type="info" message="Offline" />}

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
        {toolUse === "agent" && (
          <Flex mb="2">
            <ApplyPatchSwitch />
          </Flex>
        )}
        <Form
          disabled={disableSend}
          className={className}
          onSubmit={handleSubmit}
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
                // disabled={isStreaming}
                {...props}
                autoFocus={true}
                style={{ boxShadow: "none", outline: "none" }}
                onPaste={handlePastingFile}
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
            {config.features?.images !== false &&
              isMultimodalitySupportedForCurrentModel && <AttachFileButton />}
            {/* TODO: Reserved space for microphone button coming later on */}
            <PaperPlaneButton
              disabled={disableSend || disableInput}
              title="send"
              size="1"
              type="submit"
            />
          </Flex>
        </Form>
      </Flex>
      <FileList />

      <ChatControls
        host={config.host}
        checkboxes={checkboxes}
        onCheckedChange={onToggleCheckbox}
      />
    </Card>
  );
};
