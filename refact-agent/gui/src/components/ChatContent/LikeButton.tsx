import React from "react";
import { IconButton, Tooltip } from "@radix-ui/themes";
import classnames from "classnames";
import { knowledgeApi } from "../../services/refact/knowledge";
import { useAppSelector } from "../../hooks";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
} from "../../features/Chat";
import styles from "./LikeButton.module.css";
import { useSelector } from "react-redux";
import { selectThreadProjectOrCurrentProject } from "../../features/Chat/currentProject";

function useCreateMemory() {
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const currentProjectName = useSelector(selectThreadProjectOrCurrentProject);
  const [saveTrajectory, saveResponse] =
    knowledgeApi.useCreateNewMemoryFromMessagesMutation();

  const submitSave = React.useCallback(() => {
    void saveTrajectory({ project: currentProjectName, messages });
  }, [currentProjectName, messages, saveTrajectory]);

  const shouldShow = React.useMemo(() => {
    if (messages.length === 0) return false;
    if (isStreaming) return false;
    if (isWaiting) return false;
    return true;
  }, [messages.length, isStreaming, isWaiting]);

  return { submitSave, saveResponse, shouldShow };
}

export const LikeButton = () => {
  const { submitSave, saveResponse, shouldShow } = useCreateMemory();

  if (!shouldShow) return null;
  return (
    <Tooltip content="Save the trajectory overview to memory">
      <IconButton
        variant="ghost"
        onClick={submitSave}
        disabled={saveResponse.isLoading || saveResponse.isSuccess}
        loading={saveResponse.isLoading}
        size="2"
        className={classnames(
          saveResponse.isSuccess && styles.like__button__success,
        )}
      >
        <SaveIcon />
      </IconButton>
    </Tooltip>
  );
};

const SaveIcon: React.FC = () => {
  return (
    <svg
      height="20"
      width="20"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        fill="currentColor"
        fillRule="evenodd"
        clipRule="evenodd"
        d="M17 3H5C3.89 3 3 3.9 3 5V19C3 20.1 3.89 21 5 21H19C20.1 21 21 20.1 21 19V7L17 3ZM19 19H5V5H16.17L19 7.83V19ZM12 12C10.34 12 9 13.34 9 15C9 16.66 10.34 18 12 18C13.66 18 15 16.66 15 15C15 13.34 13.66 12 12 12ZM6 6H15V10H6V6Z"
      />
    </svg>
  );
};
