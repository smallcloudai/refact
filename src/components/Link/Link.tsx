import { FC } from "react";
import { useConfig } from "../../hooks";

interface LinkProps {
  href: string;
  children: React.ReactNode;
}

export const Link: FC<LinkProps> = ({ href, children }) => {
  const config = useConfig();

  return (
    <a
      href={href}
      style={{
        color:
          config.host === "jetbrains"
            ? config.themeProps.accentColor
            : undefined,
        filter: config.host === "jetbrains" ? "brightness(120%)" : undefined,
      }}
    >
      {children}
    </a>
  );
};
