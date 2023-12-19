import React from "react";

import { Box, Flex } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import { useOnPressedEnter } from "../../hooks/useOnPressedEnter";
import { ErrorCallout } from "../Callout";

export const ChatForm: React.FC<{
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  clearError: () => void;
  error?: string;
}> = ({ onSubmit, onClose, className, error, clearError }) => {
  const [value, setValue] = React.useState("");

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0) {
      onSubmit(trimmedValue);
      setValue(() => "");
    }
  };

  const handleEnter = useOnPressedEnter(handleSubmit);
  if (error) {
    return (
      <ErrorCallout mt="2" onClick={clearError} timeout={5000}>
        {error}
      </ErrorCallout>
    );
  }

  return (
    <Box mt="1">
      <Form className={className} onSubmit={() => handleSubmit()}>
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
      </Form>
    </Box>
  );
};
