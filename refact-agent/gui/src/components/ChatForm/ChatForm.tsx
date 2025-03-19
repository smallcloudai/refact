import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card, Text, Button, IconButton } from "@radix-ui/themes";
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
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { ComboBox } from "../ComboBox";
import { FilesPreview } from "./FilesPreview";
import { ChatControls } from "./ChatControls";
import { addCheckboxValuesToInput } from "./utils";
import { useCommandCompletionAndPreviewFiles } from "./useCommandCompletionAndPreviewFiles";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { clearError, getErrorMessage } from "../../features/Errors/errorsSlice";
import { useTourRefs } from "../../features/Tour";
import { useCheckboxes } from "./useCheckBoxes";
import { useInputValue } from "./useInputValue";
import {
  clearInformation,
  getInformationMessage,
} from "../../features/Errors/informationSlice";
import { InformationCallout } from "../Callout/Callout";
import { ToolConfirmation } from "./ToolConfirmation";
import { getPauseReasonsWithPauseStatus } from "../../features/ToolConfirmation/confirmationSlice";
import { AttachFileButton, FileList } from "../Dropzone";
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
  // selectThreadMaximumTokens,
  selectThreadToolUse,
  selectToolUse,
} from "../../features/Chat";
import { telemetryApi } from "../../services/refact";
import { push } from "../../features/Pages/pagesSlice";
import { AgentCapabilities } from "./AgentCapabilities";
import { TokensPreview } from "./TokensPreview";
// import { useUsageCounter } from "../UsageCounter/useUsageCounter";
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
  const { compressChat } = useCompressChat();
  // const currentThreadMaximumContextTokens = useAppSelector(
  //   selectThreadMaximumTokens,
  // );

  // const { isOverflown: arePromptTokensBiggerThanContext, currentThreadUsage } =
  //   useUsageCounter();

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
  }, [isOnline, isStreaming, isWaiting, preventSend, messages, allDisabled]);

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

  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

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
    if (!disableSend && trimmedValue.length > 0) {
      const valueIncludingChecks = addCheckboxValuesToInput(
        trimmedValue,
        checkboxes,
      );
      setFileInteracted(false);
      setLineSelectionInteracted(false);
      onSubmit(valueIncludingChecks);
      setValue(() => "");
      unCheckAll();
    }
  }, [
    value,
    disableSend,
    checkboxes,
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
                autoFocus={true}
                style={{ boxShadow: "none", outline: "none" }}
                onPaste={handlePastingFile}
              />
            )}
          />
          <Flex
            className={styles.textareaInteractive}
            align="center"
            justify="between"
          >
            <ThinkingButton />
            <Flex gap="2" align="center" className={styles.buttonGroup}>
              <TokensPreview currentMessageQuery={value} />
              {!unCalledTools && lastSentCompression && (
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
                  onClick={() => compressChat()}
                >
                  <ArchiveIcon />
                </IconButton>
              )}
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
                isMultimodalitySupportedForCurrentModel && <AttachFileButton />}
              {/* TODO: Reserved space for microphone button coming later on */}
              <PaperPlaneButton
                disabled={disableSend}
                title="Send message"
                size="1"
                type="submit"
              />
            </Flex>
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
