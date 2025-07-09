import React, { /*useCallback,*/ useEffect, useMemo, useState } from "react";
import { selectHost, type Config } from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import {
  useLogout,
  useAppSelector,
  useAppDispatch,
  // useStartPollingForUser,
  useEventsBusForIDE,
  useBasicStuffQuery,
} from "../../hooks";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import {
  Badge,
  Button,
  DropdownMenu,
  Flex,
  HoverCard,
  IconButton,
  // Select,
  Text,
} from "@radix-ui/themes";
import {
  HamburgerMenuIcon,
  DiscordLogoIcon,
  QuestionMarkCircledIcon,
  GearIcon,
} from "@radix-ui/react-icons";

import { PuzzleIcon } from "../../images/PuzzleIcon";
import { Coin } from "../../images";
// import { useCoinBallance } from "../../hooks/useCoinBalance";
import { isUserWithLoginMessage } from "../../services/smallcloud/types";
import {
  resetActiveGroup,
  resetActiveWorkspace,
  selectActiveGroup,
  selectActiveWorkspace,
  selectIsSkippedWorkspaceSelection,
  setSkippedWorkspaceSelection,
} from "../../features/Teams";
import { popBackTo } from "../../features/Pages/pagesSlice";
// import { useActiveTeamsGroup } from "../../hooks/useActiveTeamsGroup";

export type DropdownNavigationOptions =
  | "fim"
  | "stats"
  | "settings"
  | "hot keys"
  | "restart tour"
  | "login page"
  | "integrations"
  | "providers"
  | "";

type DropdownProps = {
  handleNavigation: (to: DropdownNavigationOptions) => void;
};

function linkForBugReports(host: Config["host"]): string {
  switch (host) {
    case "vscode":
      return "https://github.com/smallcloudai/refact-vscode/issues";
    case "jetbrains":
      return "https://github.com/smallcloudai/refact-intellij/issues";
    default:
      return "https://github.com/smallcloudai/refact-chat-js/issues";
  }
}

function linkForAccount(host: Config["host"]): string {
  switch (host) {
    case "vscode":
      return "https://app.refact.ai/profile?utm_source=plugin&utm_medium=vscode&utm_campaign=account";
    case "jetbrains":
      return "https://app.refact.ai/profile?utm_source=plugin&utm_medium=jetbrains&utm_campaign=account";
    default:
      return "https://app.refact.ai/profile?utm_source=plugin&utm_medium=unknown&utm_campaign=account";
  }
}

export const Dropdown: React.FC<DropdownProps> = ({
  handleNavigation,
}: DropdownProps) => {
  const [isOpen, setIsOpen] = useState(false);

  const refs = useTourRefs();
  const user = useBasicStuffQuery();
  const host = useAppSelector(selectHost);
  const dispatch = useAppDispatch();
  // TODO: check how much of this is still used.
  // const { maxAgentUsageAmount, currentAgentUsage } = useAgentUsage();

  const isWorkspaceSelectionSkipped = useAppSelector(
    selectIsSkippedWorkspaceSelection,
  );
  const activeWorkspace = useAppSelector(selectActiveWorkspace);
  const activeGroup = useAppSelector(selectActiveGroup);

  const coinBalance = useMemo(() => {
    const maybeWorkspaceWithCoins =
      user.data?.query_basic_stuff.workspaces.find(
        (w) => w.ws_id === activeWorkspace?.ws_id,
      );
    if (!maybeWorkspaceWithCoins) return null;
    if (!maybeWorkspaceWithCoins.have_admin) return null;
    if (maybeWorkspaceWithCoins.have_coins_exactly === 0) return null;
    return Math.round(maybeWorkspaceWithCoins.have_coins_exactly / 1000);
  }, [user.data, activeWorkspace?.ws_id]);

  const isActiveRootGroup = useMemo(() => {
    if (!activeWorkspace || !activeGroup) return false;
    return activeWorkspace.root_group_name === activeGroup.name;
  }, [activeWorkspace, activeGroup]);

  const logout = useLogout();
  // const { startPollingForUser } = useStartPollingForUser();

  // const { } = useActiveTeamsGroup();

  const bugUrl = linkForBugReports(host);
  const discordUrl = "https://www.smallcloud.ai/discord";
  const accountLink = linkForAccount(host);
  const openUrl = useOpenUrl();
  const {
    openCustomizationFile,
    openPrivacyFile,
    setLoginMessage,
    clearActiveTeamsGroupInIDE,
    clearActiveTeamsWorkspaceInIDE,
  } = useEventsBusForIDE();

  useEffect(() => {
    if (
      user.data &&
      !user.data.query_basic_stuff.workspaces.some(
        (w) => w.ws_id === activeWorkspace?.ws_id,
      )
    ) {
      // current workspace is no longer in list of cloud ones, resetting state
      clearActiveTeamsGroupInIDE();
      clearActiveTeamsWorkspaceInIDE();
      const actions = [resetActiveGroup(), resetActiveWorkspace()];
      actions.forEach((action) => dispatch(action));
    }
  }, [
    dispatch,
    clearActiveTeamsGroupInIDE,
    clearActiveTeamsWorkspaceInIDE,
    activeWorkspace,
    user.data,
  ]);

  const handleActiveGroupCleanUp = () => {
    clearActiveTeamsGroupInIDE();
    const actions = [
      resetActiveGroup(),
      resetActiveWorkspace(),
      popBackTo({ name: "history" }),
    ];
    actions.forEach((action) => dispatch(action));
  };

  // const handleProUpgradeClick = useCallback(() => {
  //   startPollingForUser();
  //   openUrl("https://refact.smallcloud.ai/pro");
  // }, [openUrl, startPollingForUser]);

  useEffect(() => {
    if (isUserWithLoginMessage(user.data)) {
      setLoginMessage(user.data.login_message);
    }
  }, [user.data, setLoginMessage]);

  const refactProductType = useMemo(() => {
    if (host === "jetbrains") return "Plugin";
    return "Extension";
  }, [host]);

  return (
    <DropdownMenu.Root open={isOpen} onOpenChange={setIsOpen}>
      <DropdownMenu.Trigger>
        <IconButton variant="outline" ref={(x) => refs.setMore(x)}>
          <HamburgerMenuIcon />
        </IconButton>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content align="center">
        {user.data && (
          <DropdownMenu.Item
            onSelect={(event) => {
              event.preventDefault();
              openUrl(accountLink);
            }}
          >
            {user.data.query_basic_stuff.fuser_id}
          </DropdownMenu.Item>
        )}

        {user.data && activeWorkspace && coinBalance && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              {/**TODO: there could be multiple source for this */}
              {coinBalance} <Coin />
              <HoverCard.Root>
                <HoverCard.Trigger>
                  <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
                </HoverCard.Trigger>
                <HoverCard.Content size="2" maxWidth="280px">
                  <Flex direction="column" gap="2">
                    <Text as="p" size="2">
                      Current coins balance on &apos;
                      {activeWorkspace.root_group_name}&apos; workspace
                    </Text>
                  </Flex>
                </HoverCard.Content>
              </HoverCard.Root>
            </Flex>
          </DropdownMenu.Label>
        )}

        {/* {user.data && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              Active plan: {user.data.inference}
            </Flex>
          </DropdownMenu.Label>
        )} */}

        {(activeWorkspace ?? activeGroup) && <DropdownMenu.Separator />}
        {activeWorkspace && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              <Text as="span" size="2">
                Active Workspace:{" "}
                <Badge>{activeWorkspace.root_group_name}</Badge>
              </Text>
              <HoverCard.Root>
                <HoverCard.Trigger>
                  <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
                </HoverCard.Trigger>
                <HoverCard.Content size="2" maxWidth="280px">
                  <Flex direction="column" gap="2">
                    <Text as="p" size="2">
                      Selected Workspace in Refact Cloud
                    </Text>
                  </Flex>
                </HoverCard.Content>
              </HoverCard.Root>
            </Flex>
          </DropdownMenu.Label>
        )}
        {activeGroup && activeWorkspace && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              <Text as="span" size="2">
                Active Group:{" "}
                <Badge color={isActiveRootGroup ? "red" : undefined}>
                  {activeGroup.name}
                </Badge>
              </Text>
              <HoverCard.Root>
                <HoverCard.Trigger>
                  <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
                </HoverCard.Trigger>
                <HoverCard.Content size="2" maxWidth="280px">
                  <Flex direction="column" gap="2">
                    <Text as="p" size="2">
                      Current selected group for knowledge
                    </Text>
                  </Flex>
                </HoverCard.Content>
              </HoverCard.Root>
            </Flex>
          </DropdownMenu.Label>
        )}
        <Flex direction="column" gap="2" mt="2" mx="2">
          {/* TODO: uncomment when plans are retrievable from flexus */}
          {/* {user.data && user.data.inference === "FREE" && (
            <Button
              color="red"
              variant="outline"
              onClick={handleProUpgradeClick}
            >
              Upgrade to PRO
            </Button>
          )} */}

          <Button
            onClick={(event) => {
              event.preventDefault();
              openUrl(discordUrl);
            }}
            variant="outline"
          >
            <Flex align="center" gap="3">
              Discord Community{" "}
              <DiscordLogoIcon
                width="20"
                height="20"
                color="var(--accent-11)"
              />
            </Flex>
          </Button>
        </Flex>

        <DropdownMenu.Separator />

        <Flex direction="column" gap="2" mb="1" mx="2">
          {isWorkspaceSelectionSkipped && (
            <Button
              onClick={() => {
                dispatch(setSkippedWorkspaceSelection(false));
                setIsOpen(false);
              }}
              variant="outline"
              color="red"
            >
              <Flex align="center" gap="3">
                Select Workspace
              </Flex>
            </Button>
          )}
        </Flex>

        <DropdownMenu.Item onSelect={() => handleNavigation("integrations")}>
          <PuzzleIcon /> Set up Agent Integrations
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("providers")}>
          <GearIcon /> Configure Providers
        </DropdownMenu.Item>

        {activeGroup && (
          <DropdownMenu.Item
            onSelect={() =>
              openUrl(`https://app.refact.ai/${activeGroup.id}/knowledge`)
            }
          >
            Manage Knowledge
          </DropdownMenu.Item>
        )}

        <DropdownMenu.Item onSelect={() => handleNavigation("settings")}>
          {refactProductType} Settings
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("hot keys")}>
          IDE Hotkeys
        </DropdownMenu.Item>

        <DropdownMenu.Item
          onSelect={() => {
            void openCustomizationFile();
          }}
        >
          Edit customization.yaml
        </DropdownMenu.Item>

        <DropdownMenu.Item
          onSelect={() => {
            void openPrivacyFile();
          }}
        >
          Edit privacy.yaml
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item onSelect={() => handleNavigation("restart tour")}>
          Restart tour
        </DropdownMenu.Item>

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            openUrl(bugUrl);
          }}
        >
          Report a bug
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("fim")}>
          Fill-in-the-middle Context
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("stats")}>
          Your Stats
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        {activeGroup && (
          <DropdownMenu.Item onSelect={handleActiveGroupCleanUp}>
            Unselect Active Group
          </DropdownMenu.Item>
        )}

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            logout();
            handleNavigation("login page");
          }}
        >
          Logout
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};
