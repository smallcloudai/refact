import React, { useCallback, useMemo } from "react";
import { Box, Flex, Spinner } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { useAppSelector, useAppDispatch, useCapsForToolUse } from "../../hooks";
import {
  ChatHistoryItem,
  deleteChatById,
} from "../../features/History/historySlice";
import { push } from "../../features/Pages/pagesSlice";
import { restoreChat } from "../../features/Chat/Thread";
import { FeatureMenu } from "../../features/Config/FeatureMenu";
import { selectActiveGroup } from "../../features/Teams";
import { GroupTree } from "./GroupTree/";
import { ErrorCallout } from "../Callout";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import classNames from "classnames";
import { selectHost } from "../../features/Config/configSlice";
import styles from "./Sidebar.module.css";

export type SidebarProps = {
  takingNotes: boolean;
  className?: string;
  style?: React.CSSProperties;
} & Omit<
  ChatHistoryProps,
  | "history"
  | "onDeleteHistoryItem"
  | "onCreateNewChat"
  | "onHistoryItemClick"
  | "currentChatId"
>;

export const Sidebar: React.FC<SidebarProps> = ({ takingNotes, style }) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const globalError = useAppSelector(getErrorMessage);
  const maybeSelectedTeamsGroup = useAppSelector(selectActiveGroup);
  const currentHost = useAppSelector(selectHost);
  const history = useAppSelector((app) => app.history, {
    // TODO: selector issue here
    devModeChecks: { stabilityCheck: "never" },
  });

  const { data: capsData } = useCapsForToolUse();

  const onDeleteHistoryItem = useCallback(
    (id: string) => dispatch(deleteChatById(id)),
    [dispatch],
  );

  const onHistoryItemClick = useCallback(
    (thread: ChatHistoryItem) => {
      dispatch(restoreChat(thread));
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  const shouldGroupTreeBeVisible = useMemo(() => {
    return (
      capsData?.metadata?.features?.includes("knowledge") === true &&
      !maybeSelectedTeamsGroup
    );
  }, [maybeSelectedTeamsGroup, capsData?.metadata?.features]);

  return (
    <Flex style={style}>
      <FeatureMenu />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>

      {!shouldGroupTreeBeVisible ? (
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      ) : (
        <GroupTree />
      )}
      {/* TODO: duplicated */}
      {globalError && (
        <ErrorCallout
          mx="0"
          timeout={3000}
          onClick={() => dispatch(clearError())}
          className={classNames(styles.popup, {
            [styles.popup_ide]: currentHost !== "web",
          })}
          preventRetry
        >
          {globalError}
        </ErrorCallout>
      )}
    </Flex>
  );
};
