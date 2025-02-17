import React from "react";
import { Markdown } from "../Markdown";

import { Box } from "@radix-ui/themes";

type ChatInputProps = {
  children: string;
};

export const SystemInput: React.FC<ChatInputProps> = (props) => {
  return (
    <Box p="2" position="relative" width="100%" style={{ maxWidth: "100%" }}>
      <Markdown>{`🤖 ${props.children}`}</Markdown>
    </Box>
  );
};
