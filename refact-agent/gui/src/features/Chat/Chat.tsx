import React, { useEffect } from "react";
import { v4 as uuid } from "uuid";
import { Chat as ChatComponent } from "../../components/Chat";
import { subscribeToThreadMessagesThunk } from "../../services/refact/chatdb";
import { useAppDispatch } from "../../hooks";

export type ChatProps = {
  threadId?: string;
};

function useMessagesForThread(threadId: string) {
  const dispatch = useAppDispatch();
  useEffect(() => {
    const thunk = dispatch(subscribeToThreadMessagesThunk(threadId));
    return () => {
      thunk.abort(threadId);
    };
  }, [dispatch, threadId]);
}

export const Chat: React.FC<ChatProps> = (props) => {
  // TBD: does sqlite have a uuid function?
  const threadId = props.threadId ?? uuid();
  useMessagesForThread(threadId);
  return <ChatComponent key={threadId} />;
};
