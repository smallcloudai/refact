import { useCallback, useEffect, useState } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectPages, change, ChatPage } from "../../features/Pages/pagesSlice";
import { setInputValue, addInputValue } from "./actions";

export function useInputValue(): [
  string,
  React.Dispatch<React.SetStateAction<string>>,
  boolean,
  React.Dispatch<React.SetStateAction<boolean>>,
] {
  const [value, setValue] = useState<string>("");
  const [isSendImmediately, setIsSendImmediately] = useState<boolean>(false);
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
        setUpIfNotReady();
        setValue((prev) => prev + payload.value);

        if (payload.send_immediately) {
          setIsSendImmediately(payload.send_immediately);
        }
      } else if (setInputValue.match(event.data)) {
        const { payload } = event.data;
        setUpIfNotReady();
        setValue(payload.value);

        if (payload.send_immediately) {
          setIsSendImmediately(payload.send_immediately);
        }
      }
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  });

  return [value, setValue, isSendImmediately, setIsSendImmediately];
}
