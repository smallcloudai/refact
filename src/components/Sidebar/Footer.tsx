import React, { useCallback } from "react";
import {
  Flex,
  IconButton,
  Link,
  DropdownMenu,
  LinkProps,
} from "@radix-ui/themes";
import { DiscordLogoIcon, DotsVerticalIcon } from "@radix-ui/react-icons";

// import { Coin } from "../../images";

import { useAppSelector, useConfig } from "../../app/hooks";
import {
  selectHost,
  type Config,
  selectLspPort,
} from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import { useEventsBusForIDE, useGetUser, useLogout } from "../../hooks";
import { Coin } from "../../images/coin";
import styles from "./sidebar.module.css";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import { CONFIG_PATH_URL } from "../../services/refact/consts";

const LinkItem: React.FC<LinkProps> = ({ children, href }) => {
  return (
    <Flex asChild gap="1" align="center">
      <Link
        size="1"
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        underline="hover"
      >
        {children}
      </Link>
    </Flex>
  );
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

export type DropdownNavigationOptions =
  | "fim"
  | "stats"
  | "settings"
  | "hot keys"
  | "restart tour"
  | "cloud login"
  | "";

type SettingsProps = {
  handleNavigation: (to: DropdownNavigationOptions) => void;
};
const Settings: React.FC<SettingsProps> = ({ handleNavigation }) => {
  const refs = useTourRefs();
  const user = useGetUser();
  const host = useAppSelector(selectHost);
  const logout = useLogout();
  const { addressURL } = useConfig();

  const bugUrl = linkForBugReports(host);
  const accountLink = linkForAccount(host);
  const openUrl = useOpenUrl();
  const { openFile } = useEventsBusForIDE();

  const port = useAppSelector(selectLspPort);
  const getCustomizationPath = useCallback(async () => {
    const previewEndpoint = `http://127.0.0.1:${port}${CONFIG_PATH_URL}`;

    const response = await fetch(previewEndpoint, {
      method: "GET",
    });
    const configPath = await response.text();
    return configPath + "/customization.yaml";
  }, [port]);

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <IconButton variant="outline" ref={(x) => refs.setMore(x)}>
          <DotsVerticalIcon />
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
              <Coin className={styles.coin} /> {user.data.metering_balance}{" "}
              coins
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

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            logout();
            handleNavigation("cloud login");
          }}
        >
          Logout
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("stats")}>
          Statistics
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            openUrl(bugUrl);
          }}
        >
          Report a bug
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("fim")}>
          FIM debug
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item
          onSelect={(event) => {
            const f = async () => {
              event.preventDefault();
              const file_name = await getCustomizationPath();
              console.log({ file_name });
              openFile({ file_name });
            };
            void f();
          }}
        >
          Edit customization.yaml
        </DropdownMenu.Item>

        {addressURL?.endsWith(".yaml") && (
          <DropdownMenu.Item
            onSelect={(event) => {
              event.preventDefault();
              openFile({ file_name: addressURL });
            }}
          >
            Edit bring your own key
          </DropdownMenu.Item>
        )}

        <DropdownMenu.Item onSelect={() => handleNavigation("hot keys")}>
          Hot Keys
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("restart tour")}>
          Restart tour
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("settings")}>
          Settings
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};

export type FooterProps = {
  handleNavigation: (to: DropdownNavigationOptions) => void;
};

export const Footer: React.FC<FooterProps> = ({ handleNavigation }) => {
  return (
    <Flex direction="column" gap="2" flexGrow="1" justify="center">
      <Flex justify="between" align="center">
        <LinkItem href="https://www.smallcloud.ai/discord">
          <DiscordLogoIcon width="10px" height="10px" /> Discord
        </LinkItem>
        <Settings handleNavigation={handleNavigation} />
      </Flex>
    </Flex>
  );
};
