import React from "react";
import {
  Flex,
  IconButton,
  Text,
  Strong,
  Link,
  DropdownMenu,
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

const Logout: React.FC = () => {
  return (
    <Flex asChild gap="1" align="center">
      <Link size="1">
        <ExitIcon /> Logout
      </Link>
    </Flex>
  );
};

const LinkItem: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <Flex asChild gap="1" align="center">
      <Link size="1">{children}</Link>
    </Flex>
  );
};

const Links: React.FC<{ hasAccount: boolean }> = ({ hasAccount }) => {
  return (
    <Text size="1">
      <Flex gap="2" justify="between">
        {hasAccount && (
          <LinkItem>
            <Link2Icon width="10px" height="10px" /> Your Account
          </LinkItem>
        )}

        <LinkItem>
          <GitHubLogoIcon width="10px" height="10px" /> Report Bug
        </LinkItem>

        <LinkItem>
          <DiscordLogoIcon width="10px" height="10px" /> Discord
        </LinkItem>
      </Flex>
    </Text>
  );
};

const Settings: React.FC = () => {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <IconButton variant="outline">
          <GearIcon />
        </IconButton>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        <DropdownMenu.Item>FIM debug</DropdownMenu.Item>
        <DropdownMenu.Item>Statistics</DropdownMenu.Item>
        <DropdownMenu.Item>Hot Keys</DropdownMenu.Item>
        <DropdownMenu.Separator />
        <DropdownMenu.Item>Settings</DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};

export type FooterProps = {
  account?: LoginInfoProps;
};

export const Footer: React.FC<FooterProps> = ({ account }) => {
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
        <Logout />
        <Settings />
      </Flex>

      <Links hasAccount={!!account} />
    </Flex>
  );
};
