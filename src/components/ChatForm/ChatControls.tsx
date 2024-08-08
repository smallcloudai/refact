import React from "react";
import { Text, Flex, HoverCard, Link } from "@radix-ui/themes";
import { Select } from "../Select";
import { type Config } from "../../features/Config/reducer";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { PromptSelect, PromptSelectProps } from "./PromptSelect";
import { Checkbox } from "../Checkbox";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

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

type CheckboxHelp = {
  text: string;
  link?: string;
  linkText?: string;
};

export type Checkbox = {
  name: string;
  label: string;
  checked: boolean;
  value?: string;
  disabled: boolean;
  fileName?: string;
  hide?: boolean;
  info?: CheckboxHelp;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (name: string, checked: boolean | string) => void;
  selectProps: CapsSelectProps;
  promptsProps: PromptSelectProps;
  host: Config["host"];
  showControls: boolean;
  useTools: boolean;
  canUseTools: boolean;
  setUseTools: (value: boolean) => void;
};

const ChatContolCheckBox: React.FC<{
  name: string;
  checked: boolean;
  disabled?: boolean;
  onCheckChange: (value: boolean | string) => void;
  label: string;
  fileName?: string;
  infoText?: string;
  href?: string;
  linkText?: string;
}> = ({
  name,
  checked,
  disabled,
  onCheckChange,
  label,
  fileName,
  infoText,
  href,
  linkText,
}) => {
  return (
    <Flex justify="between">
      <Checkbox
        size="1"
        name={name}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onCheckChange}
      >
        {" "}
        {label}
        <TruncateLeft>{fileName}</TruncateLeft>
      </Checkbox>
      {infoText && (
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon />
          </HoverCard.Trigger>
          <HoverCard.Content maxWidth="240px" size="1">
            <Flex direction="column" gap="4">
              <Text as="div" size="1">
                {infoText}
              </Text>

              {href && linkText && (
                <Text size="1">
                  Read more on our{" "}
                  <Link size="1" href={href}>
                    {linkText}
                  </Link>
                </Text>
              )}
            </Flex>
          </HoverCard.Content>
        </HoverCard.Root>
      )}
    </Flex>
  );
};

export const ChatControls: React.FC<ChatControlsProps> = ({
  checkboxes,
  onCheckedChange,
  selectProps,
  promptsProps,
  host,
  showControls,
  canUseTools,
  useTools,
  setUseTools,
}) => {
  return (
    <Flex
      pt="2"
      pb="2"
      gap="2"
      direction="column"
      className={classNames(styles.controls)}
    >
      {canUseTools && (
        <ChatContolCheckBox
          name="use_tools"
          checked={useTools}
          onCheckChange={(value) => setUseTools(!!value)}
          label="Allow model to use tools"
          infoText="Turn on when asking about your codebase. When tuned on the model can autonomously call functions to gather the best context."
          href="https://docs.refact.ai/features/ai-chat/"
          linkText="documentation"
        />
      )}

      {Object.entries(checkboxes).map(([key, checkbox]) => {
        if (host === "web" && checkbox.name === "file_upload") {
          return null;
        }
        if (checkbox.hide === true) {
          return null;
        }
        return (
          <ChatContolCheckBox
            key={key}
            name={checkbox.name}
            label={checkbox.label}
            checked={checkbox.checked}
            disabled={checkbox.disabled}
            onCheckChange={(value) => onCheckedChange(key, value)}
            infoText={checkbox.info?.text}
            href={checkbox.info?.link}
            linkText={checkbox.info?.linkText}
            fileName={checkbox.fileName}
          />
        );
      })}
      {showControls && <CapsSelect {...selectProps} />}
      {showControls && <PromptSelect {...promptsProps} />}
    </Flex>
  );
};
