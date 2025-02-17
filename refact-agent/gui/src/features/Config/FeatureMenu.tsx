import React, { useState, useEffect, useCallback } from "react";
import { Dialog, Flex, Button, Checkbox, Text } from "@radix-ui/themes";
import {
  useAppDispatch,
  useAppSelector,
  useEventsBusForIDE,
} from "../../hooks";
import { selectFeatures, changeFeature } from "./configSlice";
import { Link } from "../../components/Link";

const useInputEvent = () => {
  const [key, setKey] = useState<string | null>(null);
  useEffect(() => {
    const keyDownHandler = (event: KeyboardEvent) => setKey(event.code);
    const keyUpHandler = () => setKey(null);
    window.addEventListener("keydown", keyDownHandler);
    window.addEventListener("keyup", keyUpHandler);
    return () => {
      window.removeEventListener("keydown", keyDownHandler);
      window.removeEventListener("keyup", keyUpHandler);
    };
  }, []);

  return key;
};

const konamiCode = [
  "ArrowUp",
  "ArrowUp",
  "ArrowDown",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowLeft",
  "ArrowRight",
  "Escape",
  "Enter",
];

const useKonamiCode = () => {
  const [count, setCount] = useState(0);
  const [success, setSuccess] = useState(false);
  const key = useInputEvent();

  useEffect(() => {
    if (success) {
      return;
    } else if (document.activeElement !== document.body) {
      return;
    } else if (count === konamiCode.length) {
      setSuccess(true);
    } else if (key === konamiCode[count]) {
      setCount((n) => n + 1);
    }
  }, [key, count, success]);

  const reset = useCallback(() => {
    setSuccess(false);
    setCount(0);
  }, []);

  return { success, reset };
};

export const FeatureMenu: React.FC = () => {
  const { success, reset } = useKonamiCode();
  const dispatch = useAppDispatch();
  const features = useAppSelector(selectFeatures);

  const { openSettings } = useEventsBusForIDE();

  const handleSettingsClick = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      openSettings();
    },
    [openSettings],
  );

  // if (!success) return false;

  const keysAndValues = Object.entries(features ?? {});

  return (
    <Dialog.Root open={success} onOpenChange={reset}>
      <Dialog.Content>
        <Dialog.Title>Hidden Features Menu</Dialog.Title>
        {keysAndValues.length === 0 && (
          <Dialog.Description>No hidden features</Dialog.Description>
        )}
        <Flex direction="column" gap="3">
          {keysAndValues.map(([feature, value]) => {
            const setInSettings = feature === "ast" || feature === "vecdb";
            return (
              <Text key={feature} as="label" size="2">
                <Flex as="span" gap="2">
                  <Checkbox
                    checked={value}
                    onCheckedChange={() => {
                      dispatch(changeFeature({ feature, value: !value }));
                    }}
                    disabled={setInSettings}
                  />{" "}
                  {feature}
                  {setInSettings && (
                    <Text>
                      Option set in{" "}
                      <Link onClick={handleSettingsClick}>settings</Link>
                    </Text>
                  )}
                </Flex>
              </Text>
            );
          })}
        </Flex>

        <Flex gap="3" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">
              Close
            </Button>
          </Dialog.Close>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};
