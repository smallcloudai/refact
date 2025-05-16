import { Flex, Heading, Skeleton } from "@radix-ui/themes";
import { AnimatePresence } from "framer-motion";
import React from "react";

import { useGetToolGroupsQuery } from "../../../hooks";

import { useToolGroups } from "./useToolGroups";
import { ToolsList } from "./ToolsList";
import { ToolGroupList } from "./ToolGroupList";

export const ToolGroups: React.FC = () => {
  const { data: toolGroups, isLoading, isSuccess } = useGetToolGroupsQuery();
  const {
    toggleAllTools,
    toggleTool,
    resetSelection,
    selectToolGroup,
    selectedToolGroup,
    selectedToolGroupTools,
    someToolsEnabled,
  } = useToolGroups();

  if (isLoading || !isSuccess) return <ToolGroupsSkeleton />;

  return (
    <Flex direction="column" gap="3" style={{ overflow: "hidden" }}>
      <Heading size="3" as="h3">
        Manage Tool Groups
      </Heading>
      <AnimatePresence mode="wait" initial={false}>
        {!selectedToolGroup ? (
          <ToolGroupList groups={toolGroups} onSelect={selectToolGroup} />
        ) : (
          <>
            {selectedToolGroupTools && (
              <ToolsList
                group={selectedToolGroup}
                tools={selectedToolGroupTools}
                onToggle={toggleTool}
                onToggleAll={toggleAllTools}
                onBack={resetSelection}
                someEnabled={someToolsEnabled}
              />
            )}
          </>
        )}
      </AnimatePresence>
    </Flex>
  );
};

const ToolGroupsSkeleton: React.FC = () => {
  return (
    <Flex direction="column" gap="3" style={{ overflow: "hidden" }}>
      <Heading size="3" as="h3">
        Manage Tool Groups
      </Heading>
      <Flex direction="column" gap="1">
        {[1, 2].map((idx) => (
          <Flex key={idx} align="center" justify="between" gap="1">
            <Skeleton width="30px" height="25px" />
            <Skeleton width="100%" height="25px" />
            <Skeleton width="30px" height="25px" />
            <Skeleton width="30px" height="25px" />
          </Flex>
        ))}
      </Flex>
    </Flex>
  );
};
