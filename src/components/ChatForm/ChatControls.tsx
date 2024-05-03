import React from "react";
import { Text, Flex } from "@radix-ui/themes";
import { Select } from "../Select";
import { type Config } from "../../contexts/config-context";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { PromptSelect, PromptSelectProps } from "./PromptSelect";
import { Checkbox } from "../Checkbox";

type CapsSelectProps = {
  value: string;
  onChange: (value: string) => void;
  options: string[];
  disabled?: boolean;
};

const CapsSelect: React.FC<CapsSelectProps> = ({
  options,
  value,
  onChange,
  disabled,
}) => {
  return (
    <Flex gap="2" align="center" wrap="wrap">
      <Text size="2">Use model:</Text>
      <Select
        disabled={disabled}
        title="chat model"
        options={options}
        value={value}
        onChange={onChange}
      ></Select>
    </Flex>
  );
};

export type Checkbox = {
  name: string;
  label: string;
  checked: boolean;
  value?: string;
  disabled: boolean;
  fileName?: string;
  hide?: boolean;
  info?: string;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (name: string, checked: boolean | string) => void;
  selectProps: CapsSelectProps;
  promptsProps: PromptSelectProps;
  host: Config["host"];
};

export const ChatControls: React.FC<ChatControlsProps> = ({
  checkboxes,
  onCheckedChange,
  selectProps,
  promptsProps,
  host,
}) => {
  return (
    <Flex
      pt="2"
      pb="2"
      gap="2"
      direction="column"
      className={classNames(styles.controls)}
    >
      {Object.entries(checkboxes).map(([key, checkbox]) => {
        if (host === "web" && checkbox.name === "file_upload") {
          return null;
        }
        if (checkbox.hide === true) {
          return null;
        }
        return (
          <Checkbox
            key={key}
            size="1"
            name={checkbox.name}
            checked={checkbox.checked}
            disabled={checkbox.disabled}
            title={checkbox.info}
            onCheckedChange={(value) => onCheckedChange(key, value)}
          >
            {" "}
            {checkbox.label}
            <TruncateLeft>{checkbox.fileName}</TruncateLeft>
          </Checkbox>
        );
      })}
      <CapsSelect {...selectProps} />
      <PromptSelect {...promptsProps} />
    </Flex>
  );
};
