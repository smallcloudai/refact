import { useCallback, useEffect, useState } from "react";
import {
  useAppDispatch,
  useAppSelector,
  useSendChatRequest,
} from "../../hooks";
import { selectPages, change, ChatPage } from "../../features/Pages/pagesSlice";
import { setInputValue, addInputValue } from "./actions";

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

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (addInputValue.match(event.data)) {
        const { payload } = event.data;
        const { send_immediately, value, messages } = payload;

        setUpIfNotReady();

        if (!messages) {
          setValue((prev) => prev + value);
          setIsSendImmediately(send_immediately);
        } else {
          setIsSendImmediately(true); // if we set messages, we should create new chat immediatelly
          submit({
            maybeMessages: messages,
          });
        }
      } else if (setInputValue.match(event.data)) {
        const { payload } = event.data;
        const { send_immediately, value, messages } = payload;

        setUpIfNotReady();
        uncheckCheckboxes();

        if (!messages) {
          setValue(value);
          if (send_immediately) {
            const timeoutID = setTimeout(() => {
              setIsSendImmediately(send_immediately);
              clearTimeout(timeoutID);
            }, 100);
          }
        } else {
          setIsSendImmediately(true); // if we set messages, we should create new chat immediatelly
          submit({
            maybeMessages: messages,
          });
        }
      }
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  });

  return [value, setValue, isSendImmediately, setIsSendImmediately];
}
