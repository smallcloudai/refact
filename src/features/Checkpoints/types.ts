export type Checkpoint = {
  workspace_folder: string;
  commit_hash: string;
};

export type FileChangedStatus = "ADDED" | "MODIFIED" | "DELETED";

export type FileChanged = {
  absolute_path: string;
  relative_path: string;
  status: FileChangedStatus;
};

export type RevertedCheckpointData = {
  workspace_folder: string;
  files_changed: FileChanged[];
};

export type PreviewCheckpointsPayload = {
  checkpoints: Checkpoint[];
};

export type RestoreCheckpointsPayload = {
  checkpoints: Checkpoint[];
};

export type PreviewCheckpointsResponse = {
  reverted_to: string; // date-time in ISO format
  checkpoints_for_undo: Checkpoint[];
  reverted_changes: RevertedCheckpointData[];
  error_log: string[];
};

export type RestoreCheckpointsResponse = {
  success: boolean;
  error_log: string[];
};

export function isRestoreCheckpointsResponse(
  json: unknown,
): json is RestoreCheckpointsResponse {
  if (!json || typeof json !== "object") return false;
  if (!("success" in json) || typeof json.success !== "boolean") return false;
  if (!("error_log" in json) || !Array.isArray(json.error_log)) return false;
  return true;
}

export function isPreviewCheckpointsResponse(
  json: unknown,
): json is PreviewCheckpointsResponse {
  if (!json || typeof json !== "object") return false;

  if (!("reverted_to" in json) || typeof json.reverted_to !== "string")
    return false;

  // Check if it has the required properties
  if (!("checkpoints_for_undo" in json) || !("reverted_changes" in json))
    return false;

  // Check checkpoints_for_undo array
  if (!Array.isArray(json.checkpoints_for_undo)) return false;
  if (!json.checkpoints_for_undo.every(isCheckpoint)) return false;

  // Check reverted_changes array
  if (!Array.isArray(json.reverted_changes)) return false;
  if (!json.reverted_changes.every(isRevertedCheckpointData)) return false;

  return true;
}

// Helper type guards
function isCheckpoint(value: unknown): value is Checkpoint {
  if (!value || typeof value !== "object") return false;

  return (
    "workspace_folder" in value &&
    typeof value.workspace_folder === "string" &&
    "commit_hash" in value &&
    typeof value.commit_hash === "string"
  );
}

function isFileChanged(value: unknown): value is FileChanged {
  if (!value || typeof value !== "object") return false;

  return (
    "absolute_path" in value &&
    typeof value.absolute_path === "string" &&
    "relative_path" in value &&
    typeof value.relative_path === "string" &&
    "status" in value &&
    typeof value.status === "string" &&
    ["ADDED", "MODIFIED", "DELETED"].includes(value.status)
  );
}

function isRevertedCheckpointData(
  value: unknown,
): value is RevertedCheckpointData {
  if (!value || typeof value !== "object") return false;

  if (
    !("workspace_folder" in value) ||
    typeof value.workspace_folder !== "string"
  )
    return false;
  if (!("files_changed" in value) || !Array.isArray(value.files_changed))
    return false;

  return value.files_changed.every(isFileChanged);
}
