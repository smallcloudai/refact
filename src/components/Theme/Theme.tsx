import React, { useCallback, useEffect } from "react";
import {
  Theme as RadixTheme,
  IconButton,
  // ThemePanel
} from "@radix-ui/themes";
import { MoonIcon, SunIcon } from "@radix-ui/react-icons";
import { useDarkMode } from "usehooks-ts";
import "@radix-ui/themes/styles.css";
import "./theme-config.css";
// import { useConfig } from "../../contexts/config-context";
import { useConfig } from "../../app/hooks";
import { useMutationObserver } from "../../hooks";

export type ThemeProps = {
  children: JSX.Element;
  appearance?: "inherit" | "light" | "dark";

  accentColor?:
    | "tomato"
    | "red"
    | "ruby"
    | "crimson"
    | "pink"
    | "plum"
    | "purple"
    | "violet"
    | "iris"
    | "indigo"
    | "blue"
    | "cyan"
    | "teal"
    | "jade"
    | "green"
    | "grass"
    | "brown"
    | "orange"
    | "sky"
    | "mint"
    | "lime"
    | "yellow"
    | "amber"
    | "gold"
    | "bronze"
    | "gray";

  grayColor?: "gray" | "mauve" | "slate" | "sage" | "olive" | "sand" | "auto";
  panelBackground?: "solid" | "translucent";
  radius?: "none" | "small" | "medium" | "large" | "full";
  scaling?: "90%" | "95%" | "100%" | "105%" | "110%";
};

const ThemeWithDarkMode: React.FC<ThemeProps> = ({ children, ...props }) => {
  // TODO: use redux here
  const { isDarkMode, toggle } = useDarkMode();
  const Icon = isDarkMode ? MoonIcon : SunIcon;
  return (
    <RadixTheme {...props} appearance={isDarkMode ? "dark" : "light"}>
      <IconButton
        variant="surface"
        color="gray"
        title="toggle dark mode"
        style={{ position: "fixed", zIndex: 999, right: 15, top: 15 }}
        onClick={toggle}
      >
        <Icon />
      </IconButton>
      {/** TODO: remove this in production */}
      {/** use cmd + c to open and close */}
      {/* <ThemePanel defaultOpen={false} /> */}
      {children}
    </RadixTheme>
  );
};

export const Theme: React.FC<ThemeProps> = (props) => {
  // TODO: use redux here
  const { host, themeProps } = useConfig();
  // change this
  const [appearance, setAppearance] = React.useState<
    "dark" | "light" | "inherit"
  >("inherit");

  const handleChange = useCallback(() => {
    const maybeDark =
      document.body.classList.contains("vscode-dark") ||
      document.body.classList.contains("vscode-high-contrast");
    const maybeLight =
      document.body.classList.contains("vscode-light") ||
      document.body.classList.contains("vscode-high-contrast-light");

    if (maybeLight) {
      setAppearance("light");
    } else if (maybeDark) {
      setAppearance("dark");
    } else {
      setAppearance("inherit");
    }
  }, [setAppearance]);

  useEffect(handleChange, [handleChange]);

  useMutationObserver(document.body, handleChange, {
    attributes: true,
    characterData: false,
    childList: false,
    subtree: false,
  });

  if (host === "web") {
    return <ThemeWithDarkMode {...themeProps} {...props} />;
  }

  return <RadixTheme {...themeProps} {...props} appearance={appearance} />;
};
