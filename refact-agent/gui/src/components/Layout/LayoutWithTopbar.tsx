import React from "react";
import { BasicLayout, LayoutProps } from "./Layout";
import { Toolbar } from "../Toolbar";
import { Outlet } from "react-router";

export const LayoutWithToolbar: React.FC<LayoutProps> = (props) => {
  return (
    <BasicLayout {...props}>
      <Toolbar />
      <Outlet />
    </BasicLayout>
  );
};
