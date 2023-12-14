import React from "react";

import { Box, Flex } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";
import { TextArea } from "@radix-ui/themes";
import classNames from "classnames";
import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";

export const ChatForm: React.FC<{
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
}> = ({ onSubmit, onClose, className }) => {
  const [value, setValue] = React.useState("");

  // TODO: Maybe make a hook for this ?
  const handleEnter = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      onSubmit(value);
      setValue(() => "");
    }
  };

  return (
    <form
      className={classNames(styles.chatForm, className)}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(value);
      }}
    >
      <Box>
        <TextArea
          className={styles.textarea}
          value={value}
          onChange={(event) => {
            setValue(() => event.target.value);
          }}
          onKeyUp={handleEnter}
        />
        <Flex gap="2" className={styles.buttonGroup}>
          {onClose && (
            <BackToSideBarButton
              title="return to sidebar"
              size="1"
              onClick={onClose}
            />
          )}
          <PaperPlaneButton title="send" size="1" type="submit" />
        </Flex>
      </Box>
    </form>
  );
};
