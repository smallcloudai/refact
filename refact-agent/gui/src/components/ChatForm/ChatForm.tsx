import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card, Text, IconButton } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import {
  PaperPlaneButton,
  BackToSideBarButton,
  AgentIntegrationsButton,
  ThinkingButton,
} from "../Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  useIsOnline,
  useConfig,
  useCapsForToolUse,
  useSendChatRequest,
  useCompressChat,
  useAutoFocusOnce,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { ComboBox } from "../ComboBox";
import { FilesPreview } from "./FilesPreview";
import { CapsSelect, ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { useCommandCompletionAndPreviewFiles } from "./useCommandCompletionAndPreviewFiles";
import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  clearError,
  getErrorMessage,
  getErrorType,
} from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useAttachedFiles, useCheckboxes } from "./useCheckBoxes";
import { useInputValue } from "./useInputValue";
import {
  clearInformation,
  getInformationMessage,
  showBalanceLowCallout,
} from "../../features/Errors/informationSlice";
import {
  BallanceCallOut,
  BallanceLowInformation,
  InformationCallout,
} from "../Callout/Callout";
import { ToolConfirmation } from "./ToolConfirmation";
import { getPauseReasonsWithPauseStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { AttachImagesButton, FileList } from "../Dropzone";
import { useAttachedImages } from "../../hooks/useAttachedImages";
import {
  enableSend,
  selectChatError,
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectLastSentCompression,
  selectMessages,
  selectPreventSend,
  selectThreadToolUse,
  selectToolUse,
} from "../../features/Chat";
import { telemetryApi } from "../../services/refact";
import { push } from "../../features/Pages/pagesSlice";
import { AgentCapabilities } from "./AgentCapabilities/AgentCapabilities";
import { TokensPreview } from "./TokensPreview";
import classNames from "classnames";
import { ArchiveIcon } from "@radix-ui/react-icons";

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
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
  const config = useConfig();
  const toolUse = useAppSelector(selectToolUse);
  const globalError = useAppSelector(getErrorMessage);
  const globalErrorType = useAppSelector(getErrorType);
  const chatError = useAppSelector(selectChatError);
  const information = useAppSelector(getInformationMessage);
  const pauseReasonsWithPause = useAppSelector(getPauseReasonsWithPauseStatus);
  const [helpInfo, setHelpInfo] = React.useState<React.ReactNode | null>(null);
  const isOnline = useIsOnline();
  const { retry } = useSendChatRequest();

  const chatId = useAppSelector(selectChatId);
  const threadToolUse = useAppSelector(selectThreadToolUse);
  const messages = useAppSelector(selectMessages);
  const preventSend = useAppSelector(selectPreventSend);
  const lastSentCompression = useAppSelector(selectLastSentCompression);
  const { compressChat, compressChatRequest, isCompressing } =
    useCompressChat();
  const autoFocus = useAutoFocusOnce();
  const attachedFiles = useAttachedFiles();
  const shouldShowBalanceLow = useAppSelector(showBalanceLowCallout);

  const shouldAgentCapabilitiesBeShown = useMemo(() => {
    return threadToolUse === "agent";
  }, [threadToolUse]);

  const onClearError = useCallback(() => {
    if (messages.length > 0 && chatError) {
      retry(messages);
    }
    dispatch(clearError());
  }, [dispatch, retry, messages, chatError]);

  const caps = useCapsForToolUse();

  const allDisabled = caps.usableModelsForPlan.every((option) => {
    if (typeof option === "string") return false;
    return option.disabled;
  });

  const disableSend = useMemo(() => {
    // TODO: if interrupting chat some errors can occur
    if (allDisabled) return true;
    // if (
    //   currentThreadMaximumContextTokens &&
    //   currentThreadUsage?.prompt_tokens &&
    //   currentThreadUsage.prompt_tokens > currentThreadMaximumContextTokens
    // )
    //   return false;
    // if (arePromptTokensBiggerThanContext) return true;
    if (messages.length === 0) return false;
    return isWaiting || isStreaming || !isOnline || preventSend;
  }, [
    allDisabled,
    messages.length,
    isWaiting,
    isStreaming,
    isOnline,
    preventSend,
  ]);

  const isModelSelectVisible = useMemo(() => messages.length < 1, [messages]);

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
    setLineSelectionInteracted,
  } = useCheckboxes();

  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const [value, setValue, isSendImmediately, setIsSendImmediately] =
    useInputValue(() => unCheckAll());

  const onClearInformation = useCallback(
    () => dispatch(clearInformation()),
    [dispatch],
  );

  const { previewFiles, commands, requestCompletion } =
    useCommandCompletionAndPreviewFiles(
      checkboxes,
      attachedFiles.addFilesToInput,
    );

  const refs = useTourRefs();

  const handleSubmit = useCallback(() => {
    const trimmedValue = value.trim();
    if (!disableSend && trimmedValue.length > 0) {
      const valueWithFiles = attachedFiles.addFilesToInput(trimmedValue);
      const valueIncludingChecks = addCheckboxValuesToInput(
        valueWithFiles,
        checkboxes,
      );
      // TODO: add @files
      setLineSelectionInteracted(false);
      onSubmit(valueIncludingChecks);
      setValue(() => "");
      unCheckAll();
      attachedFiles.removeAll();
    }
  }, [
    value,
    disableSend,
    attachedFiles,
    checkboxes,
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
      if (!trimmedCommand) {
        setLineSelectionInteracted(false);
      } else {
        setLineSelectionInteracted(true);
      }

      if (trimmedCommand === "@help") {
        handleHelpInfo(helpText()); // This line has been fixed
      } else {
        handleHelpInfo(null);
      }
    },
    [handleHelpInfo, setValue, setLineSelectionInteracted],
  );

  const handleAgentIntegrationsClick = useCallback(() => {
    dispatch(push({ name: "integrations page" }));
    void sendTelemetryEvent({
      scope: `openIntegrations`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent]);

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

  if (globalError) {
    return (
      <ErrorCallout mt="2" onClick={onClearError} timeout={null}>
        {globalError}
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
    <Card mt="1" style={{ flexShrink: 0, position: "relative" }}>
      {globalErrorType === "balance" && (
        <BallanceCallOut
          mt="0"
          mb="2"
          mx="0"
          onClick={() => dispatch(clearError())}
        />
      )}
      {shouldShowBalanceLow && <BallanceLowInformation mt="0" mb="2" mx="0" />}
      {!isOnline && (
        <Callout type="info" mb="2">
          Oops, seems that connection was lost... Check your internet connection
        </Callout>
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
        {shouldAgentCapabilitiesBeShown && <AgentCapabilities />}
        <Form
          disabled={disableSend}
          className={classNames(styles.chatForm__form, className)}
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
                autoFocus={autoFocus}
                style={{ boxShadow: "none", outline: "none" }}
                onPaste={handlePastingFile}
              />
            )}
          />
          <Flex gap="1" wrap="wrap" py="1" px="2">
            {isModelSelectVisible && <CapsSelect />}

            <Flex justify="end" flexGrow="1" wrap="wrap" gap="2">
              <ThinkingButton />
              <TokensPreview
                currentMessageQuery={attachedFiles.addFilesToInput(value)}
              />
              <Flex gap="2" align="center" justify="center">
                <IconButton
                  size="1"
                  variant="ghost"
                  color={
                    lastSentCompression === "high"
                      ? "red"
                      : lastSentCompression === "medium"
                        ? "yellow"
                        : undefined
                  }
                  title="Compress chat and continue"
                  type="button"
                  onClick={() => void compressChat()}
                  disabled={
                    messages.length === 0 ||
                    isStreaming ||
                    isWaiting ||
                    unCalledTools
                  }
                  loading={compressChatRequest.isLoading || isCompressing}
                >
                  <ArchiveIcon />
                </IconButton>
                {toolUse === "agent" && (
                  <AgentIntegrationsButton
                    title="Set up Agent Integrations"
                    size="1"
                    type="button"
                    onClick={handleAgentIntegrationsClick}
                    ref={(x) => refs.setSetupIntegrations(x)}
                  />
                )}
                {onClose && (
                  <BackToSideBarButton
                    disabled={isStreaming}
                    title="Return to sidebar"
                    size="1"
                    onClick={onClose}
                  />
                )}
                {config.features?.images !== false &&
                  isMultimodalitySupportedForCurrentModel && (
                    <AttachImagesButton />
                  )}
                {/* TODO: Reserved space for microphone button coming later on */}
                <PaperPlaneButton
                  disabled={disableSend}
                  title="Send message"
                  size="1"
                  type="submit"
                />
              </Flex>
            </Flex>
          </Flex>
        </Form>
      </Flex>
      <FileList attachedFiles={attachedFiles} />

      <ChatControls
        // handle adding files
        host={config.host}
        checkboxes={checkboxes}
        onCheckedChange={onToggleCheckbox}
        attachedFiles={attachedFiles}
      />
    </Card>
  );
};
