export type Checkpoint = {
  workspace_folder: string;
  commit_hash: string;
};

export type FileChangedStatus = "A" | "M" | "D";

export type FileChanged = {
  absolute_path: string;
  relative_path: string;
  status: FileChangedStatus;
};

export type RevertedCheckpointData = {
  workspace_folder: string;
  files_changed: FileChanged[];
};

export type RestoreCheckpointsPayload = {
  checkpoints: Checkpoint[];
};

export type RestoreCheckpointsResponse = {
  checkpoints_for_undo: Checkpoint[];
  reverted_changes: RevertedCheckpointData[];
};
