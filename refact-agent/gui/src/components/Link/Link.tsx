import { FC } from "react";
import {
  type LinkProps as RadixLinkProps,
  Link as RadixLink,
} from "@radix-ui/themes";
import classNames from "classnames";

import { useConfig } from "../../hooks";
import styles from "./Link.module.css";

interface LinkProps extends RadixLinkProps {
  href?: string;
  children: React.ReactNode;
  className?: string;
  onClick?: React.MouseEventHandler<HTMLAnchorElement>;
}

export const Link: FC<LinkProps> = (props) => {
  const config = useConfig();

  return (
    <RadixLink
      className={classNames(
        styles.link,
        { [styles.jetbrains]: config.host === "jetbrains" },
        props.className,
      )}
      {...props}
    />
  );
};
