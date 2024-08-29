import { useCallback, useEffect } from "react";
import { useAppDispatch } from "../app/hooks";
import { useConfig } from "./useConfig";
import { setThemeMode } from "../features/Config/configSlice";
import { useMutationObserver } from "./useMutationObserver";

export const useAppearance = () => {
  const config = useConfig();
  const dispatch = useAppDispatch();

  const appearance = config.themeProps.appearance;

  const handleChange = useCallback(() => {
    const maybeDark =
      document.body.classList.contains("vscode-dark") ||
      document.body.classList.contains("vscode-high-contrast");
    const maybeLight =
      document.body.classList.contains("vscode-light") ||
      document.body.classList.contains("vscode-high-contrast-light");

    if (maybeLight) {
      dispatch(setThemeMode("light"));
    } else if (maybeDark) {
      dispatch(setThemeMode("dark"));
    } else {
      dispatch(setThemeMode("inherit"));
    }
  }, [dispatch]);

  useEffect(handleChange, [handleChange]);

  // TODO: remove this
  useMutationObserver(document.body, handleChange, {
    attributes: true,
    characterData: false,
    childList: false,
    subtree: false,
  });

  const toggle = useCallback(() => {
    if (appearance === "dark") return setThemeMode("light");
    if (appearance === "light") return setThemeMode("dark");
    if (appearance === "inherit") return setThemeMode("dark");
  }, [appearance]);

  return {
    appearance,
    setAppearance: setThemeMode,
    isDarkMode: appearance === "dark",
    toggle,
  };
};
