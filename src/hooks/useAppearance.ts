import { useCallback } from "react";
import { useAppDispatch } from "./useAppDispatch";
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

  useMutationObserver(document.body, handleChange, {
    attributes: true,
    characterData: false,
    childList: false,
    subtree: false,
  });

  const toggle = useCallback(() => {
    if (appearance === "dark") return dispatch(setThemeMode("light"));
    if (appearance === "light") return dispatch(setThemeMode("dark"));
    if (appearance === "inherit") return dispatch(setThemeMode("dark"));
  }, [appearance, dispatch]);

  return {
    appearance,
    setAppearance: setThemeMode,
    isDarkMode: appearance === "dark",
    toggle,
  };
};
