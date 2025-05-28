import React, { useCallback, useEffect, useMemo } from "react";
import { selectHost, type Config } from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import {
  useGetUser,
  useLogout,
  useAppSelector,
  useAppDispatch,
  useStartPollingForUser,
  useEventsBusForIDE,
  useCapsForToolUse,
} from "../../hooks";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import {
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
import { clearHistory } from "../../features/History/historySlice";
import { PuzzleIcon } from "../../images/PuzzleIcon";
import { Coin } from "../../images";
import { useCoinBallance } from "../../hooks/useCoinBalance";
import { isUserWithLoginMessage } from "../../services/smallcloud/types";
import { resetActiveGroup, selectActiveGroup } from "../../features/Teams";
import { popBackTo } from "../../features/Pages/pagesSlice";

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
      return "https://refact.smallcloud.ai/account?utm_source=plugin&utm_medium=vscode&utm_campaign=account";
    case "jetbrains":
      return "https://refact.smallcloud.ai/account?utm_source=plugin&utm_medium=jetbrains&utm_campaign=account";
    default:
      return "https://refact.smallcloud.ai/account";
  }
}

export const Dropdown: React.FC<DropdownProps> = ({
  handleNavigation,
}: DropdownProps) => {
  const refs = useTourRefs();
  const user = useGetUser();
  const host = useAppSelector(selectHost);
  const activeGroup = useAppSelector(selectActiveGroup);
  const dispatch = useAppDispatch();
  // TODO: check how much of this is still used.
  // const { maxAgentUsageAmount, currentAgentUsage } = useAgentUsage();
  const coinBalance = useCoinBallance();
  const logout = useLogout();
  const { startPollingForUser } = useStartPollingForUser();
  const { data: capsData } = useCapsForToolUse();

  const bugUrl = linkForBugReports(host);
  const discordUrl = "https://www.smallcloud.ai/discord";
  const accountLink = linkForAccount(host);
  const openUrl = useOpenUrl();
  const {
    openCustomizationFile,
    openPrivacyFile,
    setLoginMessage,
    clearActiveTeamsGroupInIDE,
  } = useEventsBusForIDE();

  const handleChatHistoryCleanUp = () => {
    dispatch(clearHistory());
  };

  const handleActiveGroupCleanUp = () => {
    clearActiveTeamsGroupInIDE();
    const actions = [resetActiveGroup(), popBackTo({ name: "history" })];
    actions.forEach((action) => dispatch(action));
  };

  const handleProUpgradeClick = useCallback(() => {
    startPollingForUser();
    openUrl("https://refact.smallcloud.ai/pro");
  }, [openUrl, startPollingForUser]);

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
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <IconButton variant="outline" ref={(x) => refs.setMore(x)}>
          <HamburgerMenuIcon />
        </IconButton>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        {user.data && (
          <DropdownMenu.Item
            onSelect={(event) => {
              event.preventDefault();
              openUrl(accountLink);
            }}
          >
            {user.data.account}
          </DropdownMenu.Item>
        )}

        {user.data && (
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
                      Current balance
                    </Text>
                  </Flex>
                </HoverCard.Content>
              </HoverCard.Root>
            </Flex>
          </DropdownMenu.Label>
        )}
        {user.data && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              Active plan: {user.data.inference}
            </Flex>
          </DropdownMenu.Label>
        )}
        {/* {user.data && user.data.workspaces.length > 0 && (
          <DropdownMenu.Label style={{ height: "unset" }}>
            <Flex
              align="stretch"
              mt="1"
              gap="1"
              direction="column"
              width="100%"
            >
              <Flex align="center" gap="1">
                <Text as="span" size="2">
                  Active workspace:
                </Text>
                <HoverCard.Root>
                  <HoverCard.Trigger>
                    <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
                  </HoverCard.Trigger>
                  <HoverCard.Content size="2" maxWidth="280px">
                    <Flex direction="column" gap="2">
                      <Text as="p" size="2">
                        Selected workspace in Team Server
                      </Text>
                    </Flex>
                  </HoverCard.Content>
                </HoverCard.Root>
              </Flex>
              <Select.Root
                size="1"
                value={activeWorkspace?.workspace_name}
                onValueChange={(value) => {
                  const workspace = user.data?.workspaces.find(
                    (w) => w.workspace_name === value,
                  );
                  if (workspace) {
                    handleSetActiveGroup(workspace);
                  }
                }}
              >
                <Select.Trigger placeholder="Choose a workspace" />
                <Select.Content position="popper">
                  {user.data.workspaces.map((w) => (
                    <Select.Item value={w.workspace_name} key={w.workspace_id}>
                      {w.workspace_name}
                    </Select.Item>
                  ))}
                </Select.Content>
              </Select.Root>
            </Flex>
          </DropdownMenu.Label>
        )} */}
        <Flex direction="column" gap="2" mt="2" mx="2">
          {user.data && user.data.inference === "FREE" && (
            <Button
              color="red"
              variant="outline"
              onClick={handleProUpgradeClick}
            >
              Upgrade to PRO
            </Button>
          )}

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

        <DropdownMenu.Item onSelect={() => handleNavigation("integrations")}>
          <PuzzleIcon /> Set up Agent Integrations
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("providers")}>
          <GearIcon /> Configure Providers
        </DropdownMenu.Item>

        {capsData?.metadata?.features?.includes("knowledge") && (
          <DropdownMenu.Item
            // TODO: get real URL from cloud inference
            onSelect={() => openUrl("https://test-teams.smallcloud.ai/")}
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

        <DropdownMenu.Item onSelect={handleChatHistoryCleanUp}>
          Clear Chat History
        </DropdownMenu.Item>

        <DropdownMenu.Item
          onSelect={handleActiveGroupCleanUp}
          disabled={activeGroup === null}
        >
          Unselect Active Group
        </DropdownMenu.Item>

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
