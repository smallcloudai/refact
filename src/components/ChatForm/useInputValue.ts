import { useCallback, useEffect, useState } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectPages, change, ChatPage } from "../../features/Pages/pagesSlice";
import { setInputValue, addInputValue } from "./actions";

export function useInputValue(): [
  string,
  React.Dispatch<React.SetStateAction<string>>,
] {
  const [value, setValue] = useState<string>("");
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
        setValue((prev) => prev + payload);
      } else if (setInputValue.match(event.data)) {
        setUpIfNotReady();
        setValue(event.data.payload);
      }
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  });

  return [value, setValue];
}
