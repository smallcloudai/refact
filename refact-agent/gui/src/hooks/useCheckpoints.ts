import { useCallback, useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import {
  selectCheckpointsMessageIndex,
  selectIsCheckpointsPopupIsVisible,
  selectIsUndoingCheckpoints,
  selectLatestCheckpointResult,
  selectShouldNewChatBeStarted,
  setCheckpointsErrorLog,
  setIsCheckpointsPopupIsVisible,
  setIsUndoingCheckpoints,
  setLatestCheckpointResult,
  setShouldNewChatBeStarted,
} from "../features/Checkpoints/checkpointsSlice";
import { useAppDispatch } from "./useAppDispatch";
import { useRestoreCheckpoints } from "./useRestoreCheckpoints";
import { Checkpoint, FileChanged } from "../features/Checkpoints/types";
import { isUserMessage, telemetryApi } from "../services/refact";
import { usePreviewCheckpoints } from "./usePreviewCheckpoints";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { selectConfig } from "../features/Config/configSlice";
import {
  resetThread,
  selectMessagesFromEndNode,
} from "../features/ThreadMessages";

// TODO: how will check points works?
export const useCheckpoints = () => {
  const dispatch = useAppDispatch();
  const messages = useAppSelector(selectMessagesFromEndNode);

  const configIdeHost = useAppSelector(selectConfig).host;

  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const { setForceReloadFileByPath } = useEventsBusForIDE();

  const { restoreChangesFromCheckpoints, isLoading: isRestoring } =
    useRestoreCheckpoints();
  const { previewChangesFromCheckpoints, isLoading: isPreviewing } =
    usePreviewCheckpoints();
  const isCheckpointsPopupVisible = useAppSelector(
    selectIsCheckpointsPopupIsVisible,
  );
  const isUndoingCheckpoints = useAppSelector(selectIsUndoingCheckpoints);

  const latestRestoredCheckpointsResult = useAppSelector(
    selectLatestCheckpointResult,
  );

  const { reverted_changes, reverted_to, error_log } =
    latestRestoredCheckpointsResult;

  const shouldNewChatBeStarted = useAppSelector(selectShouldNewChatBeStarted);
  const maybeMessageIndex = useAppSelector(selectCheckpointsMessageIndex);

  const allChangedFiles = reverted_changes.reduce<
    (FileChanged & { workspace_folder: string })[]
  >((acc, change) => {
    const filesWithWorkspace = change.files_changed.map((file) => ({
      ...file,
      workspace_folder: change.workspace_folder,
    }));
    return [...acc, ...filesWithWorkspace];
  }, []);

  const wereFilesChanged = useMemo(() => {
    return allChangedFiles.length > 0;
  }, [allChangedFiles]);

  const shouldCheckpointsPopupBeShown = useMemo(() => {
    return isCheckpointsPopupVisible && !isUndoingCheckpoints;
  }, [isCheckpointsPopupVisible, isUndoingCheckpoints]);

  const handleUndo = useCallback(() => {
    void sendTelemetryEvent({
      scope: `rollbackChanges/undo`,
      success: true,
      error_message: "",
    });
    dispatch(setIsUndoingCheckpoints(true));
  }, [dispatch, sendTelemetryEvent]);

  const handlePreview = useCallback(
    async (checkpoints: Checkpoint[] | null, messageIndex: number) => {
      if (!checkpoints) return;
      const amountOfUserMessages = messages.filter(isUserMessage);
      const firstUserMessage = amountOfUserMessages[0];
      try {
        const previewedChanges =
          await previewChangesFromCheckpoints(checkpoints).unwrap();
        void sendTelemetryEvent({
          scope: `rollbackChanges/preview`,
          success: true,
          error_message: "",
        });
        const actions = [
          dispatch(setIsUndoingCheckpoints(false)),
          setLatestCheckpointResult({
            ...previewedChanges,
            current_checkpoints: checkpoints,
            messageIndex,
          }),
          setIsCheckpointsPopupIsVisible(true),
          setShouldNewChatBeStarted(
            messageIndex === messages.indexOf(firstUserMessage),
          ),
        ];
        actions.forEach((action) => dispatch(action));
      } catch (error) {
        void sendTelemetryEvent({
          scope: `rollbackChanges/failed`,
          success: false,
          error_message: `rollback: failed to preview from checkpoints. checkpoints ${JSON.stringify(
            checkpoints,
          )}`,
        });
      }
    },
    [dispatch, previewChangesFromCheckpoints, sendTelemetryEvent, messages],
  );

  const handleFix = useCallback(async () => {
    try {
      const response = await restoreChangesFromCheckpoints(
        latestRestoredCheckpointsResult.current_checkpoints,
      ).unwrap();
      if (response.success) {
        void sendTelemetryEvent({
          scope: `rollbackChanges/confirmed`,
          success: true,
          error_message: "",
        });

        if (configIdeHost === "jetbrains") {
          const files =
            latestRestoredCheckpointsResult.reverted_changes.flatMap(
              (change) => change.files_changed,
            );
          files.forEach((file) => {
            setForceReloadFileByPath(file.absolute_path);
          });
        }

        dispatch(setIsCheckpointsPopupIsVisible(false));
      } else {
        dispatch(setCheckpointsErrorLog(response.error_log));
        return;
      }
      // TODO: new chat suggestion?
      if (shouldNewChatBeStarted || !maybeMessageIndex) {
        const actions = [resetThread()];
        actions.forEach((action) => dispatch(action));
      }
    } catch (error) {
      void sendTelemetryEvent({
        scope: `rollbackChanges/failed`,
        success: false,
        error_message: `rollback: failed to apply previewed changes from checkpoints. checkpoints: ${JSON.stringify(
          latestRestoredCheckpointsResult.current_checkpoints,
        )}`,
      });
    }
  }, [
    dispatch,
    sendTelemetryEvent,
    setForceReloadFileByPath,
    restoreChangesFromCheckpoints,
    configIdeHost,
    shouldNewChatBeStarted,
    maybeMessageIndex,
    latestRestoredCheckpointsResult.current_checkpoints,
    latestRestoredCheckpointsResult.reverted_changes,
  ]);

  return {
    shouldCheckpointsPopupBeShown,
    handleUndo,
    handlePreview,
    handleFix,
    isRestoring,
    isPreviewing,
    reverted_changes,
    reverted_to,
    wereFilesChanged,
    allChangedFiles,
    errorLog: error_log,
  };
};
