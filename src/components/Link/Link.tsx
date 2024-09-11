import { FC } from "react";
import { Link as RadixLink } from "@radix-ui/themes";
import { useConfig } from "../../hooks";
import styles from "./Link.module.css";
import classNames from "classnames";

interface LinkProps {
  href: string;
  children: React.ReactNode;
  className?: string;
}

export const Link: FC<LinkProps> = (props) => {
  const config = useConfig();

  return (
    <RadixLink
      {...props}
      className={classNames(
        { [styles.jetbrains]: config.host === "jetbrains" },
        props.className,
      )}
    />
  );
};
