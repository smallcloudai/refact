import React, { useEffect } from "react";
import { Flex, Button, Heading, Container } from "@radix-ui/themes";
import { linksApi, type ChatLink } from "../../services/refact/links";
import { diffApi, isUserMessage } from "../../services/refact";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
  useGetCapsQuery,
  useSendChatRequest,
} from "../../hooks";
import {
  chatModeToLspMode,
  selectChatId,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectModel,
  selectThreadToolUse,
} from "../../features/Chat";
import { popBackTo } from "../../features/Pages/pagesSlice";

function maybeConcatActionAndGoToStrings(link: ChatLink): string | undefined {
  const hasAction = "action" in link;
  const hasGoTo = "goto" in link;
  if (!hasAction && !hasGoTo) return "";
  if (hasAction && hasGoTo) return `action: ${link.action}\ngoto: ${link.goto}`;
  if (hasAction) return `action: ${link.action}`;
  return `goto: ${link.goto}`;
}

const isAbsolutePath = (path: string) => {
  const absolutePathRegex = /^(?:[a-zA-Z]:\\|\/|\\\\|\/\/).*/;
  return absolutePathRegex.test(path);
};

export const ChatLinks: React.FC = () => {
  const dispatch = useAppDispatch();
  const { queryPathThenOpenFile } = useEventsBusForIDE();
  const { submit } = useSendChatRequest();

  const [applyPatches, _applyPatchesResult] =
    diffApi.useApplyAllPatchesInMessagesMutation();

  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const maybeIntegration = useAppSelector(selectIntegration);
  const chatMode = useAppSelector(selectThreadToolUse);

  // TODO: add the model
  const caps = useGetCapsQuery();

  const model =
    useAppSelector(selectModel) || caps.data?.code_chat_default_model;

  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    //TODO: handle multiple tool calls in last assistant message
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  const handleGoTo = (goto?: string) => {
    if (!goto) return;
    // TODO:  duplicated in smart links.
    const [action, payload] = goto.split(":");

    switch (action.toLowerCase()) {
      case "editor": {
        void queryPathThenOpenFile({ file_name: payload });
        return;
      }
      case "settings": {
        const isFile = isAbsolutePath(payload);
        dispatch(
          popBackTo({
            name: "integrations page",
            // projectPath: isFile ? payload : "",
            integrationName: !isFile ? payload : "",
            integrationPath: isFile ? payload : "",
          }),
        );
        // TODO: open in the integrations
        return;
      }
      default: {
        // eslint-disable-next-line no-console
        console.log(`[DEBUG]: unexpected action, doing nothing`);
        return;
      }
    }
  };
  const handleLinkAction = (link: ChatLink) => {
    if (!("action" in link)) return;
    if (link.action === "goto" && "goto" in link) {
      handleGoTo(link.goto);
      return;
    }
    if (link.action === "patch-all") {
      void applyPatches(messages);
      return;
    }

    if (link.action === "follow-up") {
      submit(link.text);
      return;
    }

    if (link.action === "summarize-project") {
      submit(link.text, "PROJECTSUMMARY");
      return;
    }

    // if (link.action === "commit") {
    // ???
    //   return;
    // }

    // eslint-disable-next-line no-console
    console.warn(`unknown action: ${JSON.stringify(link)}`);
  };
  const handleClick = (link: ChatLink) => {
    if (!("action" in link) && "goto" in link) {
      handleGoTo(link.goto);
    } else {
      handleLinkAction(link);
    }
  };

  const [linksRequest, linksResult] = linksApi.useGetLinksForChatMutation();

  useEffect(() => {
    const isEmpty = messages.length === 0;
    const lastMessageIsUserMessage =
      !isEmpty && isUserMessage(messages[messages.length - 1]);
    if (
      !isStreaming &&
      !isWaiting &&
      !unCalledTools &&
      !lastMessageIsUserMessage &&
      model
    ) {
      void linksRequest({
        chat_id: chatId,
        messages: messages,
        model,
        mode: maybeIntegration ? "CONFIGURE" : chatModeToLspMode(chatMode),
        current_config_file: maybeIntegration?.path,
      });
    }
  }, [
    chatId,
    chatMode,
    isStreaming,
    isWaiting,
    linksRequest,
    maybeIntegration,
    maybeIntegration?.path,
    messages,
    model,
    unCalledTools,
  ]);

  // TODO: waiting, errors, maybe add a title

  if (!linksResult.data || isStreaming || isWaiting || unCalledTools) {
    return null;
  }

  return (
    <Container position="relative" mt="6">
      <Heading as="h4" size="2" mb="2">
        Available Actions:{" "}
      </Heading>

      <Flex gap="2" wrap="wrap" direction="column" align="start">
        {linksResult.data.links.map((link, index) => {
          const key = `chat-link-${index}`;
          return <ChatLinkButton key={key} link={link} onClick={handleClick} />;
        })}
      </Flex>
    </Container>
  );
};

const ChatLinkButton: React.FC<{
  link: ChatLink;
  onClick: (link: ChatLink) => void;
}> = ({ link, onClick }) => {
  const title = maybeConcatActionAndGoToStrings(link);
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
    >
      {link.text}
    </Button>
  );
};
