import React, { useState } from "react";
import { Button, Flex } from "@radix-ui/themes";

import { TextArea } from "../TextArea";
import { useOnPressedEnter } from "../../hooks/useOnPressedEnter";
import { Form } from "./Form";

export const RetryForm: React.FC<{
  value: string;
  onSubmit: (value: string) => void;
  onClose: () => void;
}> = (props) => {
  const [value, onChange] = useState(props.value);
  const closeAndReset = () => {
    onChange(props.value);
    props.onClose();
  };

  const handleRetry = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0) {
      props.onSubmit(trimmedValue);
    }
  };

  const onPressedEnter = useOnPressedEnter(handleRetry);

  return (
    <Form
      onSubmit={(event) => {
        event.preventDefault();
        handleRetry();
      }}
    >
      <TextArea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyUp={onPressedEnter}
      />
      <Flex
        align="center"
        justify="center"
        gap="1"
        direction="row"
        p="2"
        style={{
          backgroundColor: "var(--color-surface)",
        }}
      >
        <Button color="grass" variant="surface" size="1" type="submit">
          Submit
        </Button>
        <Button
          variant="surface"
          color="tomato"
          size="1"
          onClick={closeAndReset}
        >
          Cancel
        </Button>
      </Flex>
    </Form>
  );
};
