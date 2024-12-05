import { useCallback } from "react";
import type { FC } from "react";
import type {
  LspChatMessage,
  SmartLink as SmartLinkType,
} from "../../services/refact";
import {
  type OpenFilePayload,
  useAppDispatch,
  useEventsBusForIDE,
} from "../../hooks";
import { formatMessagesForChat } from "../../features/Chat/Thread/utils";
import { clearInformation } from "../../features/Errors/informationSlice";
import { newIntegrationChat } from "../../features/Chat";
import { push } from "../../features/Pages/pagesSlice";
import { Button, DropdownMenu } from "@radix-ui/themes";
import { AppDispatch } from "../../app/store";

const handleGotoAction = (
  sl_goto: string,
  queryPathThenOpenFile: (file: OpenFilePayload) => Promise<void>,
) => {
  const [action, payload] = sl_goto.split(":");
  switch (action.toLowerCase()) {
    // TODO: could be possible to share it between Marc's implementation
    case "editor":
      void queryPathThenOpenFile({ file_name: payload });
      break;
    case "setting":
      // Handling SETTING smartlink action
      break;
    default:
      // For unexpected actions
      break;
  }
};

const handleChatAction = (
  sl_chat: LspChatMessage[],
  dispatch: AppDispatch,
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
    }),
  );
  dispatch(push({ name: "chat" }));
};

export const SmartLink: FC<{
  smartlink: SmartLinkType;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
  isSmall?: boolean;
  isDockerSmartlink?: boolean;
}> = ({
  smartlink,
  integrationName,
  integrationPath,
  integrationProject,
  isSmall = false,
  isDockerSmartlink = false,
}) => {
  const dispatch = useAppDispatch();

  const { queryPathThenOpenFile } = useEventsBusForIDE();

  const { sl_goto, sl_chat } = smartlink;

  const handleClick = useCallback(() => {
    if (sl_goto) {
      handleGotoAction(sl_goto, queryPathThenOpenFile);
      return;
    }
    if (sl_chat) {
      handleChatAction(
        sl_chat,
        dispatch,
        integrationName,
        integrationPath,
        integrationProject,
      );
    }
  }, [
    sl_goto,
    sl_chat,
    dispatch,
    integrationName,
    integrationPath,
    queryPathThenOpenFile,
    integrationProject,
  ]);

  const title = sl_chat?.reduce<string[]>((acc, cur) => {
    if (typeof cur.content === "string")
      return [...acc, `${cur.role}: ${cur.content}`];
    return acc;
  }, []);

  const smartlinkElement = isDockerSmartlink ? (
    <DropdownMenu.Item
      onClick={handleClick}
      title={title ? title.join("\n") : ""}
    >
      {smartlink.sl_label} 🪄
    </DropdownMenu.Item>
  ) : (
    <Button
      size={isSmall ? "1" : "2"}
      onClick={handleClick}
      title={title ? title.join("\n") : ""}
      color="gray"
      type="button"
      variant="outline"
    >
      {smartlink.sl_label}
      {smartlink.sl_chat ? " 🪄" : ""}
    </Button>
  );

  return <>{smartlinkElement}</>;
};
