import React from "react";

import { Box, Flex } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

import { PaperPlaneButton, BackToSideBarButton } from "../Buttons/Buttons";
import { TextArea } from "../TextArea";
import { Form } from "./Form";
import { useOnPressedEnter } from "../../hooks/useOnPressedEnter";
import { ErrorCallout } from "../Callout";
import { ChatCapsState } from "../../hooks/useEventBusForChat";
import { Select } from "../Select/Select";

const CapsSelect: React.FC<{
  value: string;
  onChange: (event: React.ChangeEvent<HTMLSelectElement>) => void;
  options: string[];
}> = ({ options, value, onChange }) => {
  return (
    <Select
      title="chat model"
      options={options}
      value={value}
      onChange={onChange}
    ></Select>
  );
};

export const ChatForm: React.FC<{
  onSubmit: (str: string) => void;
  onClose?: () => void;
  className?: string;
  clearError: () => void;
  error: string | null;
  caps: ChatCapsState;
  model: string;
  onSetChatModel: (model: string) => void;
  canChangeModel: boolean;
  isStreaming: boolean;
}> = ({
  onSubmit,
  onClose,
  className,
  error,
  clearError,
  caps,
  model,
  onSetChatModel,
  canChangeModel,
  isStreaming,
}) => {
  const [value, setValue] = React.useState("");

  const handleSubmit = () => {
    const trimmedValue = value.trim();
    if (trimmedValue.length > 0 && !isStreaming) {
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
      {canChangeModel && (
        <CapsSelect
          value={model || caps.default_cap}
          onChange={(event) => onSetChatModel(event.target.value)}
          options={caps.available_caps}
        />
      )}
      <Form
        disabled={isStreaming}
        className={className}
        onSubmit={() => handleSubmit()}
      >
        <TextArea
          disabled={isStreaming}
          value={value}
          onChange={(event) => {
            setValue(() => event.target.value);
          }}
          onKeyUp={handleEnter}
        />
        <Flex gap="2" className={styles.buttonGroup}>
          {onClose && (
            <BackToSideBarButton
              disabled={isStreaming}
              title="return to sidebar"
              size="1"
              onClick={onClose}
            />
          )}
          <PaperPlaneButton
            disabled={isStreaming}
            title="send"
            size="1"
            type="submit"
          />
        </Flex>
      </Form>
    </Box>
  );
};
