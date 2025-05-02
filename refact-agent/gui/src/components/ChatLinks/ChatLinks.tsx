import React from "react";
import { Button } from "@radix-ui/themes";
import { type ChatLink } from "../../services/refact/links";
import { useLinksFromLsp } from "../../hooks";
import { Spinner } from "@radix-ui/themes";
import { TruncateRight } from "../Text/TruncateRight";

import styles from "./ChatLinks.module.css";
import { useCoinBallance } from "../../hooks/useCoinBalance";

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
  const balance = useCoinBallance();
  if (streaming) return null;

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
          disabled={balance <= 0}
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
          ? "You have no coins left to use Refact's AI features. Please top up your balance"
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
