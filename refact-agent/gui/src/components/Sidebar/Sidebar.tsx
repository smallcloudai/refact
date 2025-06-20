import React from "react";
import { Box, Flex, Spinner } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch } from "../../hooks";

import { FeatureMenu } from "../../features/Config/FeatureMenu";
import { GroupTree } from "./GroupTree/";
import { ErrorCallout } from "../Callout";
import { getErrorMessage, clearError } from "../../features/Errors/errorsSlice";
import classNames from "classnames";
import { selectHost } from "../../features/Config/configSlice";
import styles from "./Sidebar.module.css";
import { ThreadList } from "../../features/ThreadList/ThreadList";
import { useActiveTeamsGroup } from "../../hooks/useActiveTeamsGroup";

export type SidebarProps = {
  takingNotes: boolean;
  className?: string;
  style?: React.CSSProperties;
};

export const Sidebar: React.FC<SidebarProps> = ({ takingNotes, style }) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const globalError = useAppSelector(getErrorMessage);
  const currentHost = useAppSelector(selectHost);

  const { groupSelectionEnabled } = useActiveTeamsGroup();

  return (
    <Flex style={style}>
      <FeatureMenu />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>

      {!groupSelectionEnabled ? <ThreadList /> : <GroupTree />}
      {/* TODO: duplicated */}
      {globalError && (
        <ErrorCallout
          mx="0"
          timeout={3000}
          onClick={() => dispatch(clearError())}
          className={classNames(styles.popup, {
            [styles.popup_ide]: currentHost !== "web",
          })}
          preventRetry
        >
          {globalError}
        </ErrorCallout>
      )}
    </Flex>
  );
};
