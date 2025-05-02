import { useCallback } from "react";
import { v4 as uuidv4 } from "uuid";

import { LspChatMessage } from "../services/refact/chat";
import { formatMessagesForChat } from "../features/Chat/Thread/utils";
import { useAppDispatch } from "./useAppDispatch";
import { clearInformation } from "../features/Errors/informationSlice";
import { newIntegrationChat } from "../features/Chat/Thread/actions";
import { push } from "../features/Pages/pagesSlice";
import { useGoToLink } from "./useGoToLink";

export function useSmartLinks() {
  const dispatch = useAppDispatch();
  const { handleGoTo } = useGoToLink();
  const handleSmartLink = useCallback(
    (
      sl_chat: LspChatMessage[],
      integrationName: string,
      integrationPath: string,
      integrationProject: string,
    ) => {
      const messages = formatMessagesForChat(sl_chat);
      dispatch(clearInformation());
      dispatch(
        newIntegrationChat({
          integration: {
            name: integrationName,
            path: integrationPath,
            project: integrationProject,
          },
          messages,
          request_attempt_id: uuidv4(),
        }),
      );
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  return {
    handleSmartLink,
    handleGoTo,
  };
}
