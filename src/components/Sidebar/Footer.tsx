import React, { useCallback } from "react";
import {
  Flex,
  IconButton,
  Link,
  DropdownMenu,
  LinkProps,
} from "@radix-ui/themes";
import {
  ExitIcon,
  DiscordLogoIcon,
  DotsVerticalIcon,
} from "@radix-ui/react-icons";

// import { Coin } from "../../images";

import { useAppSelector } from "../../app/hooks";
import { selectHost, type Config } from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import { useGetUser, useLogout } from "../../hooks";
import { Coin } from "../../images/coin";
import styles from "./sidebar.module.css";

const Logout: React.FC<{
  onClick: React.MouseEventHandler<HTMLAnchorElement>;
}> = ({ onClick }) => {
  return (
    <Flex asChild gap="1" align="center">
      <Link
        onClick={onClick}
        size="1"
        style={{ cursor: "var(--cursor-link)" }}
        underline="hover"
      >
        <ExitIcon /> Logout
      </Link>
    </Flex>
  );
};

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
  | "";

type SettingsProps = {
  handleNavigation: (to: DropdownNavigationOptions) => void;
};
const Settings: React.FC<SettingsProps> = ({ handleNavigation }) => {
  const refs = useTourRefs();
  const user = useGetUser();
  const host = useAppSelector(selectHost);

  const bugUrl = linkForBugReports(host);
  const accountLink = linkForAccount(host);

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
              window.open(accountLink, "_blank");
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

        <DropdownMenu.Item onSelect={() => handleNavigation("stats")}>
          Statistics
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            window.open(bugUrl, "_blank");
          }}
        >
          Report a bug
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("fim")}>
          FIM debug
        </DropdownMenu.Item>

        <DropdownMenu.Separator />
        {/* TODO: enable these */}
        {/* <DropdownMenu.Item hidden>Edit customization.yaml</DropdownMenu.Item>

        <DropdownMenu.Item hidden>
          Edit Bring-Your-Own-Key.yaml
        </DropdownMenu.Item> */}

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
  const logout = useLogout();
  const handleLogout = useCallback(
    (e: React.MouseEvent<HTMLAnchorElement>) => {
      e.preventDefault();
      logout();
    },
    [logout],
  );
  return (
    <Flex direction="column" gap="2" flexGrow="1" justify="center">
      <Flex justify="between" align="center">
        <Flex gap="2" direction="column">
          <Logout onClick={handleLogout} />
          <LinkItem href="https://www.smallcloud.ai/discord">
            <DiscordLogoIcon width="10px" height="10px" /> Discord
          </LinkItem>
        </Flex>
        <Settings handleNavigation={handleNavigation} />
      </Flex>
    </Flex>
  );
};
