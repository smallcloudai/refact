import React from "react";

import { Box, Flex } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import { useOnPressedEnter } from "../../hooks/useOnPressedEnter";

export const ChatForm: React.FC<{
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
}> = ({ onSubmit, onClose, className }) => {
  const [value, setValue] = React.useState("");

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0) {
      onSubmit(trimmedValue);
      setValue(() => "");
    }
  };

  const handleEnter = useOnPressedEnter(handleSubmit);

  return (
    <Form className={className} onSubmit={() => handleSubmit()}>
      <Box>
        <TextArea
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
    </Form>
  );
};
