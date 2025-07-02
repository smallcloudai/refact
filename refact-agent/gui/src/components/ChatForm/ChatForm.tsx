import React, { useCallback, useEffect, useMemo } from "react";

import { Flex, Card, Text } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import {
  useOnPressedEnter,
  useIsOnline,
  useConfig,
  useAutoFocusOnce,
} from "../../hooks";
import { ErrorCallout, Callout } from "../Callout";
import { ComboBox } from "../ComboBox";
import { FilesPreview } from "./FilesPreview";
import { ChatControls } from "./ChatControls";
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
import { FileList } from "../Dropzone";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectThreadMessagesIsEmpty,
  selectToolConfirmationRequests,
} from "../../features/ThreadMessages";
import { AgentCapabilities } from "./AgentCapabilities/AgentCapabilities";
import { TokensPreview } from "./TokensPreview";
import classNames from "classnames";

import { ExpertSelect } from "../../features/ExpertsAndModels/Experts";
import { ModelsForExpert } from "../../features/ExpertsAndModels";

export type ChatFormProps = {
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
};

export const ChatForm: React.FC<ChatFormProps> = ({
  onSubmit,
  onClose,
  className,
}) => {
  const dispatch = useAppDispatch();
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  // const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
  const config = useConfig();

  const globalError = useAppSelector(getErrorMessage);
  const globalErrorType = useAppSelector(getErrorType);
  // const chatError = useAppSelector(selectChatError);
  const information = useAppSelector(getInformationMessage);
  const [helpInfo, setHelpInfo] = React.useState<React.ReactNode | null>(null);
  const isOnline = useIsOnline();
  const toolConfirmationRequests = useAppSelector(
    selectToolConfirmationRequests,
  );
  // const { retry } = useSendChatRequest();

  // const threadToolUse = useAppSelector(selectThreadToolUse);
  const messagesAreEmpty = useAppSelector(selectThreadMessagesIsEmpty);
  // TODO: compression removed?
  // const { compressChat, compressChatRequest, isCompressing } =
  //   useCompressChat();
  const autoFocus = useAutoFocusOnce();
  const attachedFiles = useAttachedFiles();
  const shouldShowBalanceLow = useAppSelector(showBalanceLowCallout);

  // const shouldAgentCapabilitiesBeShown = useMemo(() => {
  //   return threadToolUse === "agent";
  // }, [threadToolUse]);

  const onClearError = useCallback(() => {
    // if (messages.length > 0 && chatError) {
    //   retry(messages);
    // }
    dispatch(clearError());
  }, [dispatch]);

  // const caps = useCapsForToolUse();

  // const allDisabled = caps.usableModelsForPlan.every((option) => {
  //   if (typeof option === "string") return false;
  //   return option.disabled;
  // });

  const disableSend = useMemo(() => {
    // TODO: if interrupting chat some errors can occur
    // if (allDisabled) return true;
    // if (
    //   currentThreadMaximumContextTokens &&
    //   currentThreadUsage?.prompt_tokens &&
    //   currentThreadUsage.prompt_tokens > currentThreadMaximumContextTokens
    // )
    //   return false;
    // if (arePromptTokensBiggerThanContext) return true;
    if (messagesAreEmpty) return false;
    return isWaiting || isStreaming || !isOnline;
  }, [
    // allDisabled,
    isOnline,
    isStreaming,
    isWaiting,
    messagesAreEmpty,
  ]);

  // const { processAndInsertImages } = useAttachedImages();
  // TODO: disable pasting file
  // const handlePastingFile = useCallback(
  //   (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
  //     if (!isMultimodalitySupportedForCurrentModel) return;
  //     const files: File[] = [];
  //     const items = event.clipboardData.items;
  //     for (const item of items) {
  //       if (item.kind === "file") {
  //         const file = item.getAsFile();
  //         file && files.push(file);
  //       }
  //     }
  //     if (files.length > 0) {
  //       event.preventDefault();
  //       processAndInsertImages(files);
  //     }
  //   },
  //   [processAndInsertImages, isMultimodalitySupportedForCurrentModel],
  // );

  const {
    checkboxes,
    onToggleCheckbox,
    unCheckAll,
    setLineSelectionInteracted,
  } = useCheckboxes();

  // const [sendTelemetryEvent] =
  //   telemetryApi.useLazySendTelemetryChatEventQuery();

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

  // const handleAgentIntegrationsClick = useCallback(() => {
  //   dispatch(push({ name: "integrations page" }));
  //   void sendTelemetryEvent({
  //     scope: `openIntegrations`,
  //     success: true,
  //     error_message: "",
  //   });
  // }, [dispatch, sendTelemetryEvent]);

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

  if (toolConfirmationRequests.length > 0) {
    return (
      <ToolConfirmation toolConfirmationRequests={toolConfirmationRequests} />
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
        {/* {shouldAgentCapabilitiesBeShown && <AgentCapabilities />} */}
        <AgentCapabilities />
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
                // onPaste={handlePastingFile}
              />
            )}
          />
          <Flex gap="1" wrap="wrap" py="1" px="2">
            <ExpertSelect
              disabled={isStreaming || isWaiting || !messagesAreEmpty}
            />
            <ModelsForExpert
              disabled={isStreaming || isWaiting || !messagesAreEmpty}
            />

            <Flex justify="end" flexGrow="1" wrap="wrap" gap="2">
              {/* <ThinkingButton /> */}
              <TokensPreview
                currentMessageQuery={attachedFiles.addFilesToInput(value)}
              />
              <Flex gap="2" align="center" justify="center">
                {/* <IconButton
                  size="1"
                  variant="ghost"
                  // TODO: last sent compression?
                  // color={
                  //   lastSentCompression === "high"
                  //     ? "red"
                  //     : lastSentCompression === "medium"
                  //       ? "yellow"
                  //       : undefined
                  // }
                  title="Compress chat and continue"
                  type="button"
                  onClick={() => void compressChat()}
                  disabled={messagesAreEmpty || isStreaming || isWaiting}
                  loading={compressChatRequest.isLoading || isCompressing}
                >
                  <ArchiveIcon />
                </IconButton> */}
                {/* {toolUse === "agent" && (
                  <AgentIntegrationsButton
                    title="Set up Agent Integrations"
                    size="1"
                    type="button"
                    onClick={handleAgentIntegrationsClick}
                    ref={(x) => refs.setSetupIntegrations(x)}
                  />
                )} */}
                {onClose && (
                  <BackToSideBarButton
                    disabled={isStreaming}
                    title="Return to sidebar"
                    size="1"
                    onClick={onClose}
                  />
                )}
                {/** TODO: multi modality */}
                {/* {config.features?.images !== false &&
                  isMultimodalitySupportedForCurrentModel && (
                    <AttachImagesButton />
                  )} */}
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
