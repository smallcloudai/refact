import React from "react";
import { Container, Box, HoverCard } from "@radix-ui/themes";
import { Markdown } from "./ContextFiles";
import styles from "./ChatContent.module.css";
import { Small } from "../Text";
import { ScrollArea } from "../ScrollArea";

export type PlainTextProps = {
  children: string;
};

export const PlainText: React.FC<PlainTextProps> = ({ children }) => {
  const [open, setOpen] = React.useState(false);
  const text = "```text\n" + children + "\n```";
  return (
    <Container position="relative">
      <HoverCard.Root onOpenChange={setOpen} open={open}>
        <HoverCard.Trigger>
          <Box>
            <Small className={styles.file}>📝 Plain text </Small>
          </Box>
        </HoverCard.Trigger>
        <ScrollArea scrollbars="both" asChild>
          <HoverCard.Content
            size="1"
            maxHeight="50vh"
            maxWidth="90vw"
            avoidCollisions
          >
            <Markdown>{text}</Markdown>
          </HoverCard.Content>
        </ScrollArea>
      </HoverCard.Root>
    </Container>
  );
};
