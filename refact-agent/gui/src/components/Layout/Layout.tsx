import React, { useMemo } from "react";
import { Flex } from "@radix-ui/themes";
import styles from "./Layout.module.css";
import classNames from "classnames";
import { selectHost } from "../../features/Config/configSlice";
import { useAppSelector } from "../../hooks";
import { Outlet } from "react-router";

export type LayoutProps = {
  children?: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
};

export const BasicLayout: React.FC<LayoutProps> = ({
  children,
  className,
  style,
}) => {
  const host = useAppSelector(selectHost);
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
      className={classNames(styles.Layout, className)}
      style={style}
    >
      {children}
    </Flex>
  );
};

export const Layout: React.FC<LayoutProps> = (props) => {
  return (
    <BasicLayout {...props}>
      <Outlet />
    </BasicLayout>
  );
};
