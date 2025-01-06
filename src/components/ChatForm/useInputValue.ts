import { useCallback, useEffect, useState } from "react";
import {
  useAppDispatch,
  useAppSelector,
  useSendChatRequest,
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
  const { submit } = useSendChatRequest();
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
        debugRefact(`[DEBUG]: receiving event setInputValue/addInputValue`);
        setUpIfNotReady();

        if (payload.messages) {
          debugRefact(`[DEBUG]: payload messages: `, payload.messages);
          setIsSendImmediately(true);
          submit({
            maybeMessages: payload.messages,
          });
          return;
        }
      }

      if (addInputValue.match(event.data)) {
        const { payload } = event.data;
        const { send_immediately, value } = payload;
        setValue((prev) => prev + value);
        setIsSendImmediately(send_immediately);
        return;
      }

      if (setInputValue.match(event.data)) {
        const { payload } = event.data;
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
