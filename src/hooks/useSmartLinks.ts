import { useCallback } from "react";
import { LspChatMessage } from "../services/refact/chat";
import { formatMessagesForChat } from "../features/Chat/Thread/utils";
import { useAppDispatch } from "./useAppDispatch";
import {
  clearInformation,
  setInformation,
} from "../features/Errors/informationSlice";
import { newIntegrationChat } from "../features/Chat/Thread/actions";
import { push } from "../features/Pages/pagesSlice";
import { useGoToLink } from "./useGoToLink";
import { useAgentUsage } from "./useAgentUsage";

export function useSmartLinks() {
  const dispatch = useAppDispatch();
  const { aboveUsageLimit } = useAgentUsage();
  const { handleGoTo } = useGoToLink();
  const handleSmartLink = useCallback(
    (
      sl_chat: LspChatMessage[],
      integrationName: string,
      integrationPath: string,
      integrationProject: string,
    ) => {
      const messages = formatMessagesForChat(sl_chat);
      if (aboveUsageLimit) {
        const action = setInformation(
          "You have exceeded the FREE usage limit, upgrade to PRO or switch to EXPLORE mode.",
        );
        dispatch(action);
        return;
      }
      dispatch(clearInformation());
      dispatch(
        newIntegrationChat({
          integration: {
            name: integrationName,
            path: integrationPath,
            project: integrationProject,
          },
          messages,
        }),
      );
      dispatch(push({ name: "chat" }));
    },
    [dispatch, aboveUsageLimit],
  );

  return {
    handleSmartLink,
    handleGoTo,
  };
}
