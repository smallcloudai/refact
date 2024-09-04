import React, { useCallback } from "react";
import {
  selectHost,
  type Config,
  selectLspPort,
} from "../../features/Config/configSlice";
import { useTourRefs } from "../../features/Tour";
import {
  useConfig,
  useEventsBusForIDE,
  useGetUser,
  useLogout,
  useAppSelector,
} from "../../hooks";
import { useOpenUrl } from "../../hooks/useOpenUrl";
import { CONFIG_PATH_URL } from "../../services/refact/consts";
import { DropdownMenu, Flex, IconButton } from "@radix-ui/themes";
import { HamburgerMenuIcon } from "@radix-ui/react-icons";
import { Coin } from "../../images";

export type DropdownNavigationOptions =
  | "fim"
  | "stats"
  | "settings"
  | "hot keys"
  | "restart tour"
  | "cloud login"
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
              <Coin /> {user.data.metering_balance} coins
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
          Your Stats
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item
          onSelect={(event) => {
            event.preventDefault();
            openUrl(bugUrl);
          }}
        >
          Report a bug...
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("restart tour")}>
          Restart tour
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("fim")}>
          Fill-in-the-middle Context
        </DropdownMenu.Item>

        <DropdownMenu.Separator />

        <DropdownMenu.Item
          onSelect={(event) => {
            const f = async () => {
              event.preventDefault();
              const file_name = await getCustomizationPath();
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
          Hot Keys...
        </DropdownMenu.Item>

        <DropdownMenu.Item onSelect={() => handleNavigation("settings")}>
          Settings...
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  );
};
