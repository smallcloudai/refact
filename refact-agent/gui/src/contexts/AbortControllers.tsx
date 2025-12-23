import React, { createContext, useState } from "react";

type AboutFunction = (reason?: string) => void;

type AbortControllerContext = {
  addAbortController: (key: string, fn: AboutFunction) => void;
  abort: (key: string, reason?: string) => void;
  removeController: (key: string) => void;
};

export const AbortControllerContext =
  createContext<null | AbortControllerContext>(null);

export const AbortControllerProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const [abortControllers, setAbortControllers] = useState<
    Record<string, (reason?: string) => void>
  >({});

  const addAbortController: AbortControllerContext["addAbortController"] = (
    key,
    fn,
  ) => {
    setAbortControllers((prev) => ({ ...prev, [key]: fn }));
  };

  const removeController = (key: string) =>
    setAbortControllers((prev) => {
      return Object.entries(prev)
        .filter(([k]) => k !== key)
        .reduce((acc, [k, v]) => ({ ...acc, [k]: v }), {});
    });

  const abort = (key: string, reason?: string) => {
    if (key in abortControllers) {
      const fn = abortControllers[key];
      fn(reason ?? "aborted");
      removeController(key);
    }
  };

  return (
    <AbortControllerContext.Provider
      value={{ addAbortController, removeController, abort }}
    >
      {children}
    </AbortControllerContext.Provider>
  );
};
