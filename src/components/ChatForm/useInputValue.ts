import { useEffect, useState } from "react";
import { createAction } from "@reduxjs/toolkit";

export const addInputValue = createAction<string>("textarea/add");
export const setInputValue = createAction<string>("textarea/replace");

export function useInputValue(): [
  string,
  React.Dispatch<React.SetStateAction<string>>,
] {
  const [value, setValue] = useState<string>("");

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (addInputValue.match(event.data)) {
        const { payload } = event.data;
        setValue((prev) => prev + payload);
      } else if (setInputValue.match(event.data)) {
        setValue(event.data.payload);
      }
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  });

  return [value, setValue];
}
