import { useCallback } from "react";

import { LspChatMessage } from "../services/refact/chat";
import { formatMessagesForChat } from "../features/Chat/Thread/utils";
import { useAppDispatch } from "./useAppDispatch";
import { clearInformation } from "../features/Errors/informationSlice";

import { push } from "../features/Pages/pagesSlice";
import { useGoToLink } from "./useGoToLink";

export function useSmartLinks() {
  const dispatch = useAppDispatch();
  const { handleGoTo } = useGoToLink();
  const handleSmartLink = useCallback(
    (
      sl_chat: LspChatMessage[],
      _integrationName: string,
      _integrationPath: string,
      _integrationProject: string,
    ) => {
      const _messages = formatMessagesForChat(sl_chat);
      dispatch(clearInformation());
      // TODO: how do we handle integration chats?
      // dispatch(
      //   newIntegrationChat({
      //     integration: {
      //       name: integrationName,
      //       path: integrationPath,
      //       project: integrationProject,
      //     },
      //     messages,
      //     request_attempt_id: uuidv4(),
      //   }),
      // );
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  return {
    handleSmartLink,
    handleGoTo,
  };
}
