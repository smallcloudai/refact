import React, { useCallback } from "react";

import { Flex, Text, Link } from "@radix-ui/themes";

import { useConfig, useEventsBusForIDE } from "../../hooks";
import { useAppSelector } from "../../hooks";

import { currentTipOfTheDay } from "../../features/TipOfTheDay";

const TipOfTheDay: React.FC = () => {
  const tip = useAppSelector(currentTipOfTheDay);

  return (
    <Text>
      💡 <b>Tip of the day</b>: {tip}
    </Text>
  );
};

export const PlaceHolderText: React.FC = () => {
  const config = useConfig();
  const hasVecDB = config.features?.vecdb ?? false;
  const hasAst = config.features?.ast ?? false;
  const { openSettings } = useEventsBusForIDE();

  const handleOpenSettings = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      openSettings();
    },
    [openSettings],
  );

  if (config.host === "web") {
    <Flex direction="column" gap="4">
      <Text>Welcome to Refact chat!</Text>;
      <TipOfTheDay />
    </Flex>;
  }

  if (!hasVecDB && !hasAst) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on VecDB and AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasVecDB) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on VecDB in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasAst) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  }

  return (
    <Flex direction="column" gap="4">
      <Text>Welcome to Refact chat.</Text>
      <TipOfTheDay />
    </Flex>
  );
};
