import React from "react";
import { Box } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";
import { ScrollArea } from "../ScrollArea";

export const Form: React.FC<
  React.PropsWithChildren<{
    className?: string;
    onSubmit: React.FormEventHandler<HTMLFormElement>;
    disabled?: boolean;
  }>
> = ({ onSubmit, ...props }) => {
  return (
    <Box mt="1" className={styles.chatForm}>
      <ScrollArea scrollbars="vertical" className={styles.chatForm_ScrollArea}>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onSubmit(event);
          }}
          {...props}
        />
      </ScrollArea>
    </Box>
  );
};
