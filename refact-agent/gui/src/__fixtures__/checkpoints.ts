import { CheckpointsMeta } from "../features/Checkpoints/checkpointsSlice";

export const STUB_PREVIEWED_CHECKPOINT_DATA: CheckpointsMeta["latestCheckpointResult"] =
  {
    reverted_to: "2025-01-24T17:44:08Z",
    current_checkpoints: [],
    checkpoints_for_undo: [],
    error_log: [],
    reverted_changes: [
      {
        files_changed: [
          {
            absolute_path: "test.txt",
            relative_path: "test.txt",
            status: "MODIFIED",
          },
          {
            absolute_path:
              "\\?\\\\C:\\\\Users\\\\andre\\\\Desktop\\\\work\\\\refact.ai\\\\refact-lsp\\\\src\\\\main.rs",
            relative_path: "src/main.rs",
            status: "DELETED",
          },
        ],
        workspace_folder: "refact-lsp",
      },
    ],
  };

export const STUB_PREVIEWED_CHECKPOINTS_STATE: CheckpointsMeta = {
  isVisible: true,
  isUndoing: false,
  restoringUserMessageIndex: null,
  shouldNewChatBeStarted: false,
  latestCheckpointResult: STUB_PREVIEWED_CHECKPOINT_DATA,
};

export const STUB_RESTORED_CHECKPOINTS_STATE_WITH_NO_CHANGES: CheckpointsMeta =
  {
    isVisible: true,
    isUndoing: false,
    restoringUserMessageIndex: null,
    shouldNewChatBeStarted: false,
    latestCheckpointResult: {
      reverted_to: "2024-01-20T15:30:00Z",
      checkpoints_for_undo: [],
      current_checkpoints: [],
      reverted_changes: [],
      error_log: [],
    },
  };
