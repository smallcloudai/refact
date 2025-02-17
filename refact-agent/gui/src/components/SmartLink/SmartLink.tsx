import { useCallback } from "react";
import type { FC } from "react";
import type { SmartLink as SmartLinkType } from "../../services/refact";
import { Button, DropdownMenu } from "@radix-ui/themes";
import { useSmartLinks } from "../../hooks";
import styles from "./SmartLink.module.css";

export const SmartLink: FC<{
  smartlink: SmartLinkType;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
  isSmall?: boolean;
  isDockerSmartlink?: boolean;
  shouldBeDisabled?: boolean;
}> = ({
  smartlink,
  integrationName,
  integrationPath,
  integrationProject,
  isSmall = false,
  isDockerSmartlink = false,
  shouldBeDisabled,
}) => {
  const { handleGoTo, handleSmartLink } = useSmartLinks();

  const { sl_goto, sl_chat } = smartlink;

  const handleClick = useCallback(() => {
    if (sl_goto) {
      handleGoTo({ goto: sl_goto });
      return;
    }
    if (sl_chat) {
      handleSmartLink(
        sl_chat,
        integrationName,
        integrationPath,
        integrationProject,
      );
    }
  }, [
    sl_goto,
    sl_chat,
    handleGoTo,
    handleSmartLink,
    integrationName,
    integrationPath,
    integrationProject,
  ]);

  const title = sl_chat?.reduce<string[]>((acc, cur) => {
    if (typeof cur.content === "string")
      return [...acc, `${cur.role}: ${cur.content}`];
    return acc;
  }, []);

  const smartlinkElement = isDockerSmartlink ? (
    <DropdownMenu.Item
      onClick={handleClick}
      title={title ? title.join("\n") : ""}
    >
      ✨ {smartlink.sl_label}
    </DropdownMenu.Item>
  ) : (
    <Button
      size={isSmall ? "1" : "2"}
      onClick={handleClick}
      title={title ? title.join("\n") : ""}
      color="gray"
      type="button"
      variant="outline"
      className={styles.magicButton}
      disabled={shouldBeDisabled}
    >
      {smartlink.sl_chat ? "✨ " : ""}
      {smartlink.sl_label}
    </Button>
  );

  return <>{smartlinkElement}</>;
};
