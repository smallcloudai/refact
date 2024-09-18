import React from "react";
import { createPortal } from "react-dom";
import { useConfig } from "../../hooks";
import { Theme } from "../Theme";

export type PortalProps = { element?: HTMLElement; children: JSX.Element };
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
