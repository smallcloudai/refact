import React from "react";
import { createPortal } from "react-dom";
import { useConfig } from "../../contexts/config-context";
import { Theme } from "../Theme";

export type PortalProps = React.PropsWithChildren<{ element?: HTMLElement }>;
export const Portal: React.FC<PortalProps> = ({
  children,
  element = document.body,
}) => {
  const config = useConfig();
  return createPortal(
    <Theme {...config.themeProps}>{children}</Theme>,
    element,
  );
};
