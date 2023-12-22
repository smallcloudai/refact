import React from "react";
import { Theme as RadixTheme } from "@radix-ui/themes";
import { useDarkMode } from "usehooks-ts";
import "@radix-ui/themes/styles.css";

export const Theme: React.FC<React.ComponentProps<typeof RadixTheme>> = (
  props,
) => {
  const { isDarkMode } = useDarkMode();
  return <RadixTheme {...props} appearance={isDarkMode ? "dark" : "light"} />;
};
