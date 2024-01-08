import React from "react";
import {
  Theme as RadixTheme,
  IconButton,
  // ThemePanel
} from "@radix-ui/themes";
import { MoonIcon, SunIcon } from "@radix-ui/react-icons";
import { useDarkMode } from "usehooks-ts";
import "@radix-ui/themes/styles.css";
import "./theme-config.css";

export const Theme: React.FC<React.ComponentProps<typeof RadixTheme>> = ({
  children,
  ...props
}) => {
  // TODO: this isn't needed when in an IDE
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
