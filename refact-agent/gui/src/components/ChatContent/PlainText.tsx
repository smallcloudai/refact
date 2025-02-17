import React from "react";
import { Container, Box, HoverCard } from "@radix-ui/themes";
import { Markdown } from "./ContextFiles";
import styles from "./ChatContent.module.css";
import { Small } from "../Text";
import { ScrollArea } from "../ScrollArea";
import { FileTextIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

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
            <Small
              as="span"
              className={classNames(styles.file, styles.file_with_icon)}
            >
              <FileTextIcon width="1em" height="1em" />
              Plain text
            </Small>
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
