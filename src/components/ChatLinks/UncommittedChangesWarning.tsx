import React from "react";
import { useGetLinksFromLsp } from "../../hooks";
import { Markdown } from "../Markdown";
import { Flex, Separator } from "@radix-ui/themes";

export const UncommittedChangesWarning: React.FC = () => {
  const linksRequest = useGetLinksFromLsp();

  if (!linksRequest.data?.uncommited_changes_warning) return false;

  return (
    <Flex py="4" gap="4" direction="column" justify="between">
      <Separator size="4" />
      <Markdown>{linksRequest.data.uncommited_changes_warning}</Markdown>
    </Flex>
  );
};
