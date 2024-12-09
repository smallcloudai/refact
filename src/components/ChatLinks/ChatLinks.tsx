import React, { useEffect } from "react";
import { Flex, Button, Container, Box } from "@radix-ui/themes";
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
  selectChatId,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectModel,
  selectThreadMode,
  setIntegrationData,
} from "../../features/Chat";
import { popBackTo } from "../../features/Pages/pagesSlice";
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
  const threadMode = useAppSelector(selectThreadMode);

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
            integrationName:
              !isFile && payload !== "DEFAULT"
                ? payload
                : maybeIntegration?.name,
            integrationPath: isFile ? payload : maybeIntegration?.path,
            projectPath: maybeIntegration?.project,
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
      void applyPatches(messages).then(() => {
        if ("goto" in link) {
          handleGoTo(link.goto);
        }
      });
      return;
    }

    if (link.action === "follow-up") {
      submit(link.text);
      return;
    }

    if (link.action === "summarize-project") {
      if ("current_config_file" in link && link.current_config_file) {
        dispatch(setIntegrationData({ path: link.current_config_file }));
        // set the integration data
      }
      submit(link.text, "PROJECT_SUMMARY");
      return;
    }

    // if (link.action === "commit") {
    //   // TODO: there should be an endpoint for this
    //   void applyPatches(messages).then(() => {
    //     if ("goto" in link && link.goto) {
    //       handleGoTo(link.goto);
    //     }
    //   });

    //   return;
    // }

    // eslint-disable-next-line no-console
    console.warn(`unknown action: ${JSON.stringify(link)}`);
  };
  const handleClick = (link: ChatLink) => {
    handleLinkAction(link);
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
        mode: threadMode,
        current_config_file: maybeIntegration?.path,
      });
    }
  }, [
    chatId,
    isStreaming,
    isWaiting,
    linksRequest,
    maybeIntegration,
    maybeIntegration?.path,
    messages,
    model,
    threadMode,
    unCalledTools,
  ]);

  // TODO: waiting, errors, maybe add a title

  if (isStreaming || isWaiting || unCalledTools) {
    return null;
  }

  const Wrapper = messages.length === 0 ? Box : Container;

  if (linksResult.isLoading) {
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
              <ChatLinkButton key={key} link={link} onClick={handleClick} />
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
