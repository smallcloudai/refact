import React, { useState } from "react";
import { Button } from "@radix-ui/themes";

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
      <Button type="submit">Submit</Button>
      <Button onClick={closeAndReset}>Cancel</Button>
    </Form>
  );
};
