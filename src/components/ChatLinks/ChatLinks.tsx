import React from "react";
import { Flex, Button, Container, Box } from "@radix-ui/themes";
import { type ChatLink } from "../../services/refact/links";
import { useAppSelector, useLinksFromLsp } from "../../hooks";
import { selectMessages } from "../../features/Chat";
import { Spinner } from "@radix-ui/themes";
import { TruncateRight } from "../Text/TruncateRight";

function maybeConcatActionAndGoToStrings(link: ChatLink): string | undefined {
  const hasAction = "action" in link;
  const hasGoTo = "goto" in link;
  if (!hasAction && !hasGoTo) return "";
  if (hasAction && hasGoTo) return `action: ${link.action}\ngoto: ${link.goto}`;
  if (hasAction) return `action: ${link.action}`;
  return `goto: ${link.goto}`;
}

export const ChatLinks: React.FC = () => {
  const messages = useAppSelector(selectMessages);
  const { linksResult, handleLinkAction, streaming } = useLinksFromLsp();

  if (streaming) return null;

  const Wrapper = messages.length === 0 ? Box : Container;

  // TODO: waiting, errors, maybe add a title

  if (linksResult.isLoading || linksResult.isFetching) {
    return (
      <Wrapper position="relative" mt="6">
        <Button variant="surface" disabled>
          <Spinner loading />
          Checking for actions
        </Button>
      </Wrapper>
    );
  }

  if (linksResult.data && linksResult.data.links.length > 0) {
    return (
      <Wrapper position="relative" mt="6">
        <Flex gap="2" wrap="wrap" direction="column" align="start">
          {linksResult.data.links.map((link, index) => {
            const key = `chat-link-${index}`;
            return (
              <ChatLinkButton
                key={key}
                link={link}
                onClick={handleLinkAction}
              />
            );
          })}
        </Flex>
      </Wrapper>
    );
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
