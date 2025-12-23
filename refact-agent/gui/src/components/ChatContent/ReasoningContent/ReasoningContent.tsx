import { Box, Flex, Heading, Text } from "@radix-ui/themes";
import React from "react";

import { Markdown } from "../../Markdown";

import styles from "./ReasoningContent.module.css";

type ReasoningContentProps = {
  reasoningContent: string;
  onCopyClick: (text: string) => void;
};

export const ReasoningContent: React.FC<ReasoningContentProps> = ({
  reasoningContent,
  onCopyClick,
}) => {
  return (
    <Box py="4" style={{ paddingRight: "50px" }}>
      <Flex direction="column" gap="2" className={styles.reasoningCallout}>
        <Heading as="h3" color="gray" size="2">
          Model Reasoning
        </Heading>
        <Text size="2" color="gray">
          <Markdown canHaveInteractiveElements={true} onCopyClick={onCopyClick}>
            {reasoningContent}
          </Markdown>
        </Text>
      </Flex>
    </Box>
  );
};
