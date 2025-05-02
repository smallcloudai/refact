import React from "react";
import { Button } from "@radix-ui/themes";
import { type ChatLink } from "../../services/refact/links";
import { useLinksFromLsp } from "../../hooks";
import { Spinner } from "@radix-ui/themes";
import { TruncateRight } from "../Text/TruncateRight";

import styles from "./ChatLinks.module.css";

function maybeConcatActionAndGoToStrings(link: ChatLink): string | undefined {
  const hasAction = "link_action" in link;
  const hasGoTo = "link_goto" in link;
  if (!hasAction && !hasGoTo) return "";
  if (hasAction && hasGoTo) {
    return `action: ${link.link_action}\ngoto: ${link.link_goto}`;
  }
  return `action: ${link.link_action}`;
}

export const ChatLinks: React.FC = () => {
  const { linksResult, handleLinkAction, streaming } = useLinksFromLsp();
  if (streaming) return null;

  const submittingChatActions = ["post-chat", "follow-up", "summarize-project"];

  // TODO: waiting, errors, maybe add a title

  if (linksResult.isLoading || linksResult.isFetching) {
    return (
      <Button variant="surface" disabled>
        <Spinner loading />
        Checking for actions
      </Button>
    );
  }

  if (linksResult.data && linksResult.data.links.length > 0) {
    return linksResult.data.links.map((link, index) => {
      const key = `chat-link-${index}`;
      return (
        <ChatLinkButton
          key={key}
          link={link}
          onClick={handleLinkAction}
          disabled={submittingChatActions.includes(link.link_action)}
        />
      );
    });
  }

  return null;
};

export const ChatLinkButton: React.FC<{
  link: ChatLink;
  onClick: (link: ChatLink) => void;
  disabled?: boolean;
}> = ({ link, onClick, disabled = false }) => {
  const title = link.link_tooltip ?? maybeConcatActionAndGoToStrings(link);
  const handleClick = React.useCallback(() => onClick(link), [link, onClick]);
  return (
    <Button
      // variant="classic"
      // variant="solid"
      // variant="outline"
      // variant="soft"
      // variant="ghost"

      variant="surface"
      title={
        disabled
          ? "You have reached your usage limit for the day. You can use agent again tomorrow, or upgrade to PRO."
          : title
      }
      onClick={handleClick}
      className={styles.chat_link_button}
      disabled={disabled}
    >
      <TruncateRight>{link.link_text}</TruncateRight>
    </Button>
  );
};
