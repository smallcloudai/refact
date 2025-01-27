import React, { useCallback, useMemo } from "react";
import { selectHost, type Config } from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import {
  useConfig,
  useEventsBusForIDE,
  useGetUser,
  useLogout,
  useAppSelector,
  useAppDispatch,
  useAgentUsage,
} from "../../hooks";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import { Button, DropdownMenu, Flex, IconButton } from "@radix-ui/themes";
import { HamburgerMenuIcon, DiscordLogoIcon } from "@radix-ui/react-icons";
import { clearHistory } from "../../features/History/historySlice";
//import { Coin } from "../../images";

export type DropdownNavigationOptions =
  | "fim"
  | "stats"
  | "settings"
  | "hot keys"
  | "restart tour"
  | "cloud login"
  | "integrations"
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
  const dispatch = useAppDispatch();
  const logout = useLogout();
  const { addressURL } = useConfig();
  const { startPollingForUser } = useAgentUsage();

  const bugUrl = linkForBugReports(host);
  const discordUrl = "https://www.smallcloud.ai/discord";
  const accountLink = linkForAccount(host);
  const openUrl = useOpenUrl();
  const { openBringYourOwnKeyFile, openCustomizationFile, openPrivacyFile } =
    useEventsBusForIDE();

  const handleChatHistoryCleanUp = () => {
    dispatch(clearHistory());
  };

  const handleProUpgradeClick = useCallback(() => {
    startPollingForUser();
    openUrl("https://refact.smallcloud.ai/pro");
  }, [openUrl, startPollingForUser]);

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

        {/*
        Hide coins (until coins logic is reworked)
        {user.data && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              <Coin /> {user.data.metering_balance} coins
            </Flex>
          </DropdownMenu.Label>
        )} */}
        {user.data && (
          <DropdownMenu.Label>
            <Flex align="center" gap="1">
              Active plan: {user.data.inference}
            </Flex>
          </DropdownMenu.Label>
        )}
        <Flex direction="column" gap="2" mt="1" mx="2">
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
          Set up Agent Integrations
        </DropdownMenu.Item>

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

        {addressURL?.endsWith(".yaml") && (
          <DropdownMenu.Item
            onSelect={() => {
              void openBringYourOwnKeyFile();
            }}
          >
            Edit Bring Your Own Key
          </DropdownMenu.Item>
        )}

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
          onSelect={(event) => {
            event.preventDefault();
            logout();
            handleNavigation("cloud login");
          }}
        >
          Logout
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};
