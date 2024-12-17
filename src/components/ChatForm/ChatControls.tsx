import React, { useCallback } from "react";
import { Text, Flex, HoverCard, Link } from "@radix-ui/themes";
import { Select } from "../Select";
import { type Config } from "../../features/Config/configSlice";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { PromptSelect, PromptSelectProps } from "./PromptSelect";
import { Checkbox } from "../Checkbox";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { useTourRefs } from "../../features/Tour";
import { ToolUseSwitch } from "./ToolUseSwitch";
import { ToolUse, selectToolUse, setToolUse } from "../../features/Chat/Thread";
import {
  useAppSelector,
  useAppDispatch,
  useCapsForToolUse,
  useCanUseTools,
} from "../../hooks";

const CapsSelect: React.FC = () => {
  const refs = useTourRefs();
  const caps = useCapsForToolUse();

  const allDisabled = caps.usableModelsForPlan.every((option) => {
    if (typeof option === "string") return false;
    return option.disabled;
  });

  return (
    <Flex
      gap="2"
      align="center"
      wrap="wrap"
      ref={(x) => refs.setUseModel(x)}
      style={{ alignSelf: "flex-start" }}
    >
      {/** TODO: loading state */}
      <Text size="2">Use model:</Text>

      {!caps.loading && allDisabled ? (
        <Text size="1" color="gray">
          No models available
        </Text>
      ) : (
        <Select
          disabled={caps.loading}
          title="chat model"
          options={caps.usableModelsForPlan}
          value={caps.currentModel}
          onChange={caps.setCapModel}
        ></Select>
      )}
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
  onCheckedChange: (
    name: keyof ChatControlsProps["checkboxes"],
    checked: boolean | string,
  ) => void;
  promptsProps: PromptSelectProps;
  host: Config["host"];
  showControls: boolean;
};

const ChatControlCheckBox: React.FC<{
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
        {label}
        {fileName && (
          // TODO: negative margin ?
          <Flex ml="-3px">
            <TruncateLeft>{fileName}</TruncateLeft>
          </Flex>
        )}
      </Checkbox>
      {infoText && (
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
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
  promptsProps,
  host,
  showControls,
}) => {
  const refs = useTourRefs();
  const canUseTools = useCanUseTools();
  const dispatch = useAppDispatch();
  const toolUse = useAppSelector(selectToolUse);
  const onSetToolUse = useCallback(
    (value: ToolUse) => dispatch(setToolUse(value)),
    [dispatch],
  );

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
          <Flex
            style={{
              // TODO: lots of `align` self
              alignSelf: "flex-start",
            }}
            key={key}
          >
            <ChatControlCheckBox
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
          </Flex>
        );
      })}

      {canUseTools && showControls && (
        <Flex
          ref={(x) => refs.setUseTools(x)}
          style={{ alignSelf: "flex-start" }}
        >
          <ToolUseSwitch toolUse={toolUse} setToolUse={onSetToolUse} />
        </Flex>
      )}

      {showControls && (
        <Flex style={{ alignSelf: "flex-start" }}>
          <CapsSelect />
        </Flex>
      )}
      {showControls && (
        <Flex style={{ alignSelf: "flex-start" }}>
          <PromptSelect {...promptsProps} />
        </Flex>
      )}
    </Flex>
  );
};
