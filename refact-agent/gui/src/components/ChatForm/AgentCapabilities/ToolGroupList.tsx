import React from "react";
import { motion } from "framer-motion";
import { Flex } from "@radix-ui/themes";

import { ToolGroup } from "./ToolGroup";
import { ScrollArea } from "../../ScrollArea";

import { ToolGroup as ToolGroupType } from "../../../services/refact";

export type ToolGroupListProps = {
  groups: ToolGroupType[];
  onSelect: (group: ToolGroupType | null) => void;
};

export const ToolGroupList: React.FC<ToolGroupListProps> = ({
  groups,
  onSelect,
}) => {
  return (
    <motion.div
      key="group-list"
      initial={{ opacity: 0, x: -40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.25 }}
    >
      <ScrollArea
        scrollbars="vertical"
        type="auto"
        style={{
          maxHeight: "125px",
        }}
      >
        <Flex direction="column" gap="1" pr={groups.length < 4 ? "0" : "3"}>
          {groups.map((toolGroup) => (
            <ToolGroup
              key={toolGroup.name}
              group={toolGroup}
              setSelectedToolGroup={onSelect}
            />
          ))}
        </Flex>
      </ScrollArea>
    </motion.div>
  );
};
