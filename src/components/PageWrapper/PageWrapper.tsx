import React, { useMemo } from "react";
import { Flex } from "@radix-ui/themes";
import styles from "./PageWrapper.module.css";
import classNames from "classnames";
import type { Config } from "../../features/Config/configSlice";

type PageWrapperProps = {
  children: React.ReactNode;
  host: Config["host"];
  className?: string;
  style?: React.CSSProperties;
};

export const PageWrapper: React.FC<PageWrapperProps> = ({
  children,
  className,
  host,
  style,
}) => {
  const xPadding = useMemo(() => {
    if (host === "web") return { initial: "8", xl: "9" };
    return {
      initial: "2",
      xs: "2",
      sm: "4",
      md: "8",
      lg: "8",
      xl: "9",
    };
  }, [host]);

  const yPadding = useMemo(() => {
    return host === "web" ? "5" : "2";
  }, [host]);

  return (
    <Flex
      direction="column"
      justify="between"
      flexGrow="1"
      py={yPadding}
      px={xPadding}
      className={classNames(styles.PageWrapper, className)}
      style={style}
    >
      {children}
    </Flex>
  );
};
