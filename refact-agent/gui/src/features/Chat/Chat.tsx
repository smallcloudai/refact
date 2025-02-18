import React, { useEffect } from "react";
import type { Config } from "../Config/configSlice";
import { Chat as ChatComponent } from "../../components/Chat";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  newChatAction,
  restoreChat,
  selectChatFromCacheOrHistory,
  selectMessages,
  selectThread,
} from "./Thread";
import { useNavigate, useParams } from "react-router";

function useNavigateToChat() {
  const thread = useAppSelector(selectThread);
  const dispatch = useAppDispatch();
  const navigate = useNavigate();
  const params = useParams();
  const cached = useAppSelector(selectChatFromCacheOrHistory(params.chatId));

  useEffect(() => {
    if (cached && cached.id !== thread.id) {
      // TODO: these hooks are a bit of a hack around creating a new thread, then navigating to it
      dispatch(restoreChat(cached));
    }
  }, [cached, dispatch, thread.id]);

  useEffect(() => {
    if (thread.id === params.chatId) return;
    void navigate(`/chat/${thread.id}`);
  }, [thread.id, navigate, params.chatId]);

  useEffect(() => {
    if (!params.chatId) {
      dispatch(newChatAction());
    }
  }, [dispatch, params.chatId]);
}

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  style?: React.CSSProperties;
  backFromChat: () => void;
};

export const Chat: React.FC<ChatProps> = ({
  style,
  backFromChat,
  host,
  tabbed,
}) => {
  const messages = useAppSelector(selectMessages);
  useNavigateToChat();

  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  return (
    <ChatComponent
      // style not used
      style={style}
      // host not used
      host={host}
      // tabbed not used
      tabbed={tabbed}
      // back ... can be a link
      backFromChat={backFromChat}
      unCalledTools={unCalledTools}
    />
  );
};
