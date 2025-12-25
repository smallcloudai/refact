import { useCallback, useEffect, useState } from "react";
import {
  useAppDispatch,
  useAppSelector,
  useChatActions,
} from "../../hooks";
import { selectPages, change, ChatPage } from "../../features/Pages/pagesSlice";
import { setInputValue, addInputValue } from "./actions";
import { debugRefact } from "../../debugConfig";

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
  const { submit } = useChatActions();
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
      if (addInputValue.match(event.data) || setInputValue.match(event.data)) {
        const { payload } = event.data;
        debugRefact(
          `[DEBUG]: receiving event setInputValue/addInputValue with payload:`,
          payload,
        );
        setUpIfNotReady();

        if (payload.messages && payload.messages.length > 0) {
          debugRefact(`[DEBUG]: payload messages: `, payload.messages);
          setIsSendImmediately(true);
          // Extract text from last user message if available
          const lastMsg = payload.messages[payload.messages.length - 1];
          if (lastMsg && lastMsg.role === "user") {
            let content = "";
            if (typeof lastMsg.content === "string") {
              content = lastMsg.content;
            } else if (Array.isArray(lastMsg.content)) {
              const textItem = lastMsg.content.find(
                (c: unknown): c is { type: "text"; text: string } =>
                  typeof c === "object" && c !== null && "type" in c && c.type === "text"
              );
              content = textItem?.text ?? "";
            }
            void submit(content);
          }
          return;
        }
      }

      if (addInputValue.match(event.data)) {
        const { payload } = event.data;
        debugRefact(`[DEBUG]: addInputValue triggered with:`, payload);
        const { send_immediately, value } = payload;
        setValue((prev) => {
          debugRefact(`[DEBUG]: Previous value: "${prev}", Adding: "${value}"`);
          return prev + value;
        });
        setIsSendImmediately(send_immediately);
        return;
      }

      if (setInputValue.match(event.data)) {
        const { payload } = event.data;
        debugRefact(`[DEBUG]: setInputValue triggered with:`, payload);
        const { send_immediately, value } = payload;
        uncheckCheckboxes();
        setValue(value ?? "");
        debugRefact(`[DEBUG]: setInputValue.payload: `, payload);
        setIsSendImmediately(send_immediately);
        return;
      }
    },
    [setUpIfNotReady, submit, uncheckCheckboxes],
  );

  useEffect(() => {
    window.addEventListener("message", handleEvent);

    return () => window.removeEventListener("message", handleEvent);
  }, [handleEvent]);

  return [value, setValue, isSendImmediately, setIsSendImmediately];
}
