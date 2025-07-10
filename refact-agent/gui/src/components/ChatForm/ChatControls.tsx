import React from "react";
import {
  Text,
  Flex,
  HoverCard,
  Link,
  Switch,
  Badge,
  Button,
} from "@radix-ui/themes";
import { type Config } from "../../features/Config/configSlice";
import { TruncateLeft } from "../Text";
import styles from "./ChatForm.module.css";
import classNames from "classnames";
import { Checkbox } from "../Checkbox";
import {
  ExclamationTriangleIcon,
  LockClosedIcon,
  LockOpen1Icon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";

import {
  selectPatchIsAutomatic,
  selectThreadId,
  selectToolConfirmationResponses,
} from "../../features/ThreadMessages";
import { useAppSelector } from "../../hooks";
import { useAttachedFiles } from "./useCheckBoxes";
import { graphqlQueriesAndMutations } from "../../services/graphql";

export const ApplyPatchSwitch: React.FC = () => {
  const chatId = useAppSelector(selectThreadId);
  const isPatchAutomatic = useAppSelector(selectPatchIsAutomatic);
  const toolConfirmationResponses = useAppSelector(
    selectToolConfirmationResponses,
  );
  const [toolConfirmation, _toolConfirmationResult] =
    graphqlQueriesAndMutations.useToolConfirmationMutation();

  const handleAutomaticPatchChange = (checked: boolean) => {
    const value = checked
      ? toolConfirmationResponses.filter((res) => res !== "*")
      : [...toolConfirmationResponses, "*"];

    void toolConfirmation({
      ft_id: chatId,
      confirmation_response: JSON.stringify(value),
    });
  };

  return (
    <Flex
      gap="4"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      justify="between"
    >
      <Text size="2" mr="auto">
        Patch files without confirmation
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable automatic patch calls by Agent"
          checked={isPatchAutomatic}
          onCheckedChange={handleAutomaticPatchChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content side="top" align="end" size="1" maxWidth="280px">
            <Text weight="bold" size="2">
              Enabled
            </Text>
            <Text as="p" size="1">
              When enabled, Refact Agent will automatically apply changes to
              files without asking for your confirmation.
            </Text>
            <Text as="div" mt="2" size="2" weight="bold">
              Disabled
            </Text>
            <Text as="p" size="1">
              When disabled, Refact Agent will ask for your confirmation before
              applying any unsaved changes.
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};

// TODO: figure out how this should work
export const AgentRollbackSwitch: React.FC = () => {
  // const dispatch = useAppDispatch();
  // TODO: checkpoints
  // const isAgentRollbackEnabled = true; // useAppSelector(selectCheckpointsEnabled);

  // TODO: handle this
  // const handleAgentRollbackChange = (checked: boolean) => {
  //   dispatch(setEnabledCheckpoints(checked));
  // };

  return (
    <Flex
      gap="4"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      justify="between"
    >
      <Text size="2" mr="auto">
        Changes rollback
      </Text>
      <Flex gap="2" align="center">
        {/** TODO: figure out what is happening with checkpoints */}
        {/* <Switch
          size="1"
          title="Enable/disable changed rollback made by Agent"
          checked={isAgentRollbackEnabled}
          onCheckedChange={handleAgentRollbackChange}
        /> */}
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content side="top" align="end" size="1" maxWidth="280px">
            <Flex direction="column" gap="2">
              <Text as="p" size="1">
                When enabled, Refact Agent will automatically make snapshots of
                files between your messages
              </Text>
              <Text as="p" size="1">
                You can rollback file changes to checkpoints taken when you sent
                messages to Agent
              </Text>
              <Badge
                color="yellow"
                asChild
                style={{
                  whiteSpace: "pre-wrap",
                }}
              >
                <Flex gap="2" py="1" px="2" align="center">
                  <ExclamationTriangleIcon
                    width={16}
                    height={16}
                    style={{ flexGrow: 1, flexShrink: 0 }}
                  />
                  <Text as="p" size="1">
                    Warning: may slow down performance of Agent in large
                    projects
                  </Text>
                </Flex>
              </Badge>
            </Flex>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};

// const FollowUpsSwitch: React.FC = () => {
//   const dispatch = useAppDispatch();
//   const areFollowUpsEnabled = useAppSelector(selectAreFollowUpsEnabled);

//   const handleFollowUpsEnabledChange = (checked: boolean) => {
//     dispatch(setAreFollowUpsEnabled(checked));
//   };

//   return (
//     <Flex
//       gap="4"
//       align="center"
//       wrap="wrap"
//       flexGrow="1"
//       flexShrink="0"
//       width="100%"
//       justify="between"
//     >
//       <Text size="2" mr="auto">
//         Follow-Ups messages
//       </Text>
//       <Flex gap="2" align="center">
//         <Switch
//           size="1"
//           title="Enable/disable follow-ups messages generation by Agent"
//           checked={areFollowUpsEnabled}
//           onCheckedChange={handleFollowUpsEnabledChange}
//         />
//         <HoverCard.Root>
//           <HoverCard.Trigger>
//             <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
//           </HoverCard.Trigger>
//           <HoverCard.Content side="top" align="end" size="1" maxWidth="280px">
//             <Flex direction="column" gap="2">
//               <Text as="p" size="1">
//                 When enabled, Refact Agent will automatically generate related
//                 follow-ups to the conversation
//               </Text>
//               <Badge
//                 color="yellow"
//                 asChild
//                 style={{
//                   whiteSpace: "pre-wrap",
//                 }}
//               >
//                 <Flex gap="2" p="2" align="center">
//                   <ExclamationTriangleIcon
//                     width={16}
//                     height={16}
//                     style={{ flexGrow: 1, flexShrink: 0 }}
//                   />
//                   <Text as="p" size="1">
//                     Warning: may increase coins spending
//                   </Text>
//                 </Flex>
//               </Badge>
//             </Flex>
//           </HoverCard.Content>
//         </HoverCard.Root>
//       </Flex>
//     </Flex>
//   );
// };

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
  locked?: boolean;
};

export type ChatControlsProps = {
  checkboxes: Record<string, Checkbox>;
  onCheckedChange: (
    name: keyof ChatControlsProps["checkboxes"],
    checked: boolean | string,
  ) => void;

  host: Config["host"];
  attachedFiles: ReturnType<typeof useAttachedFiles>;
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
  locked?: boolean;
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
  locked,
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
        {locked && <LockClosedIcon opacity="0.6" />}
        {locked === false && <LockOpen1Icon opacity="0.6" />}
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
  host,
  attachedFiles,
}) => {
  return (
    <Flex
      pt="2"
      pb="2"
      gap="2"
      direction="column"
      align="start"
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
          <ChatControlCheckBox
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
            locked={checkbox.locked}
          />
        );
      })}

      {host !== "web" && (
        <Button
          title="Attach current file"
          onClick={attachedFiles.addFile}
          disabled={!attachedFiles.activeFile.name || attachedFiles.attached}
          size="1"
          radius="medium"
        >
          Attach: {attachedFiles.activeFile.name}
        </Button>
      )}
    </Flex>
  );
};
