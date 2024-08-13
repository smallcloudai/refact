import React from "react";
import {
  Flex,
  IconButton,
  Text,
  Strong,
  Link,
  DropdownMenu,
  LinkProps,
} from "@radix-ui/themes";
import {
  GearIcon,
  ReloadIcon,
  ExitIcon,
  Link2Icon,
  GitHubLogoIcon,
  DiscordLogoIcon,
} from "@radix-ui/react-icons";

import { Coin } from "../../images";
import styles from "./sidebar.module.css";
import { useConfig } from "../../app/hooks";
import type { Config } from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";

export type LoginInfoProps = {
  email: string;
  tokens: number;
  plan: string;
};
const LoginInfo: React.FC<LoginInfoProps> = ({ email, tokens, plan }) => {
  return (
    <Flex direction="column" gap="1">
      <Flex justify="between">
        <Text size="1">{email}</Text>
        <Text size="1" align="center">
          <Flex align="center" gap="1">
            <Coin className={styles.coin} /> {tokens}
          </Flex>
        </Text>
      </Flex>

      <Flex align="center" gap="1">
        <Text size="1">
          Active Plan: <Strong>{plan}</Strong>{" "}
        </Text>
        <IconButton size="1" variant="ghost" title="refresh">
          <ReloadIcon height="8px" width="8px" />
        </IconButton>
      </Flex>
    </Flex>
  );
};

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

const Links: React.FC<{ hasAccount: boolean }> = ({ hasAccount }) => {
  const { host } = useConfig();
  const bugUrl = linkForBugReports(host);
  const accountLink = linkForAccount(host);
  return (
    <Text size="1">
      <Flex gap="2" justify="between">
        {hasAccount && (
          <LinkItem href={accountLink}>
            <Link2Icon width="10px" height="10px" /> Your Account
          </LinkItem>
        )}

        <LinkItem href={bugUrl}>
          <GitHubLogoIcon width="10px" height="10px" /> Report Bug
        </LinkItem>

        <LinkItem href="https://www.smallcloud.ai/discord">
          <DiscordLogoIcon width="10px" height="10px" /> Discord
        </LinkItem>
      </Flex>
    </Text>
  );
};

type SettingsProps = {
  handleNavigation: (
    to: "fim" | "stats" | "settings" | "hot keys" | "",
  ) => void;
};
const Settings: React.FC<SettingsProps> = ({ handleNavigation }) => {
  const refs = useTourRefs();

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <IconButton variant="outline" ref={(x) => refs.setMore(x)}>
          <GearIcon />
        </IconButton>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        <DropdownMenu.Item onSelect={() => handleNavigation("fim")}>
          FIM debug
        </DropdownMenu.Item>
        <DropdownMenu.Item onSelect={() => handleNavigation("stats")}>
          Statistics
        </DropdownMenu.Item>
        <DropdownMenu.Item onSelect={() => handleNavigation("hot keys")}>
          Hot Keys
        </DropdownMenu.Item>
        <DropdownMenu.Separator />
        <DropdownMenu.Item onSelect={() => handleNavigation("settings")}>
          Settings
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};

export type FooterProps = {
  account?: LoginInfoProps;
  handleLogout: () => void;
  handleNavigation: (
    to: "fim" | "stats" | "settings" | "hot keys" | "",
  ) => void;
};

export const Footer: React.FC<FooterProps> = ({
  account,
  handleLogout,
  handleNavigation,
}) => {
  return (
    <Flex direction="column" gap="2" flexGrow="1">
      {account && (
        <LoginInfo
          email={account.email}
          tokens={account.tokens}
          plan={account.plan}
        />
      )}
      <Flex justify="between">
        <Logout
          onClick={(e) => {
            e.preventDefault();
            handleLogout();
          }}
        />
        <Settings handleNavigation={handleNavigation} />
      </Flex>

      <Links hasAccount={!!account} />
    </Flex>
  );
};
