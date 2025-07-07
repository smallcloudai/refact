import { useCallback, useEffect, useState } from "react";
import {
  useAppDispatch,
  useAppSelector,
  // useSendChatRequest,
} from "../../hooks";
import { selectPages, change, ChatPage } from "../../features/Pages/pagesSlice";
import { setInputValue } from "./actions";
import { debugRefact } from "../../debugConfig";
import { useMessageSubscription } from "../Chat/useMessageSubscription";

export function useInputValue(
  uncheckCheckboxes: () => void,
): [
  string,
  React.Dispatch<React.SetStateAction<string>>,
  boolean,
  React.Dispatch<React.SetStateAction<boolean>>,
] {
  const [value, setValue] = useState<string>("");
  const [isSendImmediately, setIsSendImmediately] = useState<boolean>(false);
  const { sendMessage, sendMultipleMessages } = useMessageSubscription();
  const dispatch = useAppDispatch();
  const pages = useAppSelector(selectPages);

  const setUpIfNotReady = useCallback(() => {
    const lastPage = pages[pages.length - 1];
    if (lastPage.name !== "chat") {
      const chatPage: ChatPage = { name: "chat" };
      dispatch(change(chatPage));
    }
  }, [dispatch, pages]);

  const handleEvent = useCallback(
    (event: MessageEvent) => {
      if (
        /* addInputValue.match(event.data) || */ setInputValue.match(event.data)
      ) {
        const { payload } = event.data;
        debugRefact(
          `[DEBUG]: receiving event setInputValue/addInputValue with payload:`,
          payload,
        );
        setUpIfNotReady();

        if (payload.messages && payload.send_immediately) {
          debugRefact(`[DEBUG]: payload messages: `, payload.messages);
          void sendMultipleMessages(payload.messages);
          return;
        }

        if (payload.value && payload.send_immediately) {
          void sendMessage(payload.value);
        } else if (payload.value) {
          debugRefact(`[DEBUG]: setInputValue triggered with:`, payload);
          uncheckCheckboxes();
          setValue(payload.value);
          debugRefact(`[DEBUG]: setInputValue.payload: `, payload);
        }
      }
    },
    [sendMessage, sendMultipleMessages, setUpIfNotReady, uncheckCheckboxes],
  );

  useEffect(() => {
    window.addEventListener("message", handleEvent);

    return () => window.removeEventListener("message", handleEvent);
  }, [handleEvent]);

  return [value, setValue, isSendImmediately, setIsSendImmediately];
}
