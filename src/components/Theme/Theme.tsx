import React from "react";
import { Theme as RadixTheme, IconButton } from "@radix-ui/themes";
import { MoonIcon, SunIcon } from "@radix-ui/react-icons";
import { useDarkMode } from "usehooks-ts";
import "@radix-ui/themes/styles.css";

export const Theme: React.FC<React.ComponentProps<typeof RadixTheme>> = ({
  children,
  ...props
}) => {
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
      {children}
    </RadixTheme>
  );
};
