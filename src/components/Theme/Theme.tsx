import React, { useEffect } from "react";
import {
  Theme as RadixTheme,
  IconButton,
  // ThemePanel
} from "@radix-ui/themes";
import { MoonIcon, SunIcon } from "@radix-ui/react-icons";
import { useDarkMode } from "usehooks-ts";
import "@radix-ui/themes/styles.css";
import "./theme-config.css";
import { useConfig } from "../../contexts/config-context";

export type ThemeProps = React.ComponentProps<typeof RadixTheme>;

const ThemeWithDarkMode: React.FC<ThemeProps> = ({ children, ...props }) => {
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
  const { host, themeProps } = useConfig();
  const [isDarkMode, setDarkMode] = React.useState(false);

  useEffect(() => {
    const maybeDark = document.body.classList.contains("vscode-dark");
    setDarkMode(() => maybeDark);
  }, []);

  if (host === "web") {
    return <ThemeWithDarkMode {...themeProps} {...props} />;
  }
  // todo make this a hook

  return (
    <RadixTheme
      {...themeProps}
      {...props}
      appearance={isDarkMode ? "dark" : "inherit"}
    />
  );
};
