import React from "react";
import { Button } from "@radix-ui/themes";
import { type ChatLink } from "../../services/refact/links";
import { useAppSelector, useLinksFromLsp } from "../../hooks";
import { Spinner } from "@radix-ui/themes";
import { TruncateRight } from "../Text/TruncateRight";
import { selectThreadToolUse } from "../../features/Chat";

function maybeConcatActionAndGoToStrings(link: ChatLink): string | undefined {
  const hasAction = "action" in link;
  const hasGoTo = "goto" in link;
  if (!hasAction && !hasGoTo) return "";
  if (hasAction && hasGoTo) return `action: ${link.action}\ngoto: ${link.goto}`;
  if (hasAction) return `action: ${link.action}`;
  return `goto: ${link.goto}`;
}

export const ChatLinks: React.FC = () => {
  const { linksResult, handleLinkAction, streaming } = useLinksFromLsp();
  const toolUse = useAppSelector(selectThreadToolUse);

  if (streaming) return null;
  if (toolUse !== "agent") return null;

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
        <ChatLinkButton key={key} link={link} onClick={handleLinkAction} />
      );
    });
  }

  return null;
};

const ChatLinkButton: React.FC<{
  link: ChatLink;
  onClick: (link: ChatLink) => void;
}> = ({ link, onClick }) => {
  const title = link.link_tooltip || maybeConcatActionAndGoToStrings(link);
  const handleClick = React.useCallback(() => onClick(link), [link, onClick]);

  return (
    <Button
      // variant="classic"
      // variant="solid"
      // variant="outline"
      // variant="soft"
      // variant="ghost"

      variant="surface"
      title={title}
      onClick={handleClick}
      style={{ maxWidth: "100%" }}
    >
      <TruncateRight>{link.text}</TruncateRight>
    </Button>
  );
};
