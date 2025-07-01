import React from "react";
import { useAppSelector } from "../../hooks";

import { Flex, Separator } from "@radix-ui/themes";
import {
  // selectIsStreaming,
  // selectIsWaiting,
  selectMessages,
  selectThreadToolUse,
} from "../../features/Chat";
import {
  selectIsStreaming,
  selectIsWaiting,
} from "../../features/ThreadMessages";
import { getErrorMessage } from "../../features/Errors/errorsSlice";
import { getInformationMessage } from "../../features/Errors/informationSlice";

export const UncommittedChangesWarning: React.FC = () => {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  // const linksRequest = useGetLinksFromLsp();
  const error = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);
  const toolUse = useAppSelector(selectThreadToolUse);
  const messages = useAppSelector(selectMessages);

  const hasCallout = React.useMemo(() => {
    return !!error || !!information;
  }, [error, information]);

  if (
    toolUse !== "agent" ||
    messages.length !== 0 ||
    hasCallout ||
    isStreaming ||
    isWaiting
  ) {
    return false;
  }

  return (
    <Flex py="4" gap="4" direction="column" justify="between">
      <Separator size="4" />
      {/* <Markdown>{linksRequest.data.uncommited_changes_warning}</Markdown> */}
    </Flex>
  );
};
