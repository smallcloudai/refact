import React from "react";
import { useAppSelector, useGetLinksFromLsp } from "../../hooks";
import { Markdown } from "../Markdown";
import { Flex, Separator } from "@radix-ui/themes";
import { selectIsStreaming, selectIsWaiting } from "../../features/Chat";

export const UncommittedChangesWarning: React.FC = () => {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const linksRequest = useGetLinksFromLsp();

  if (
    isStreaming ||
    isWaiting ||
    linksRequest.isFetching ||
    linksRequest.isLoading ||
    !linksRequest.data?.uncommited_changes_warning
  ) {
    return false;
  }

  return (
    <Flex py="4" gap="4" direction="column" justify="between">
      <Separator size="4" />
      <Markdown>{linksRequest.data.uncommited_changes_warning}</Markdown>
    </Flex>
  );
};
