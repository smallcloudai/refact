import classNames from "classnames";
import { Badge, Flex } from "@radix-ui/themes";

import styles from "./OnOffSwitch.module.css";
import React, { MouseEventHandler } from "react";

const switches = [
  { label: "On", leftRadius: true },
  { label: "Off", rightRadius: true },
];

export type OnOffSwitchProps = {
  isEnabled: boolean;
  isUnavailable?: boolean;
  isUpdating?: boolean;
  handleClick: MouseEventHandler<HTMLDivElement>;
};

export const OnOffSwitch: React.FC<OnOffSwitchProps> = ({
  isEnabled,
  isUnavailable = false,
  isUpdating = false,
  handleClick,
}) => {
  return (
    <Flex
      className={classNames(styles.switch, {
        [styles.disabled]: isUpdating,
      })}
      onClick={handleClick}
    >
      {switches.map(({ label, leftRadius }) => {
        const isOn = label === "On";
        const isActive = isOn === isEnabled;

        return (
          <Badge
            key={label}
            color={isActive && !isUpdating ? "jade" : "gray"}
            variant="soft"
            radius="medium"
            className={classNames({ [styles.unavailable]: isUnavailable })}
            style={{
              ...(leftRadius
                ? {
                    borderTopRightRadius: 0,
                    borderBottomRightRadius: 0,
                  }
                : {
                    borderTopLeftRadius: 0,
                    borderBottomLeftRadius: 0,
                  }),
            }}
          >
            {label}
          </Badge>
        );
      })}
    </Flex>
  );
};
