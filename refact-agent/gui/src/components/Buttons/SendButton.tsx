import React from "react";
import { DropdownMenu, IconButton, Flex, Badge } from "@radix-ui/themes";
import {
  PaperPlaneIcon,
  CaretDownIcon,
  ClockIcon,
  LightningBoltIcon,
} from "@radix-ui/react-icons";

type SendButtonProps = {
  disabled?: boolean;
  isStreaming?: boolean;
  queuedCount?: number;
  onSend: () => void;
  onSendImmediately: () => void;
};

export const SendButtonWithDropdown: React.FC<SendButtonProps> = ({
  disabled,
  isStreaming,
  queuedCount = 0,
  onSend,
  onSendImmediately,
}) => {
  const showDropdown = isStreaming && !disabled;

  if (!showDropdown) {
    return (
      <Flex align="center" gap="2">
        {queuedCount > 0 && (
          <Badge
            color="amber"
            size="1"
            variant="soft"
            title={`${queuedCount} message(s) queued`}
          >
            <ClockIcon width={12} height={12} />
            {queuedCount}
          </Badge>
        )}
        <IconButton
          variant="ghost"
          disabled={disabled}
          title="Send message"
          size="1"
          type="submit"
          onClick={(e) => {
            e.preventDefault();
            onSend();
          }}
        >
          <PaperPlaneIcon />
        </IconButton>
      </Flex>
    );
  }

  return (
    <Flex align="center" gap="2">
      {queuedCount > 0 && (
        <Badge
          color="amber"
          size="1"
          variant="soft"
          title={`${queuedCount} message(s) queued`}
        >
          <ClockIcon width={12} height={12} />
          {queuedCount}
        </Badge>
      )}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <IconButton
            variant="ghost"
            disabled={disabled}
            title="Send options"
            size="1"
          >
            <PaperPlaneIcon />
            <CaretDownIcon width={12} height={12} />
          </IconButton>
        </DropdownMenu.Trigger>

        <DropdownMenu.Content size="1" align="end">
          <DropdownMenu.Item onSelect={() => onSend()}>
            <ClockIcon width={14} height={14} />
            Queue message
          </DropdownMenu.Item>
          <DropdownMenu.Item onSelect={() => onSendImmediately()}>
            <LightningBoltIcon width={14} height={14} />
            Send next
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </Flex>
  );
};

export default SendButtonWithDropdown;
