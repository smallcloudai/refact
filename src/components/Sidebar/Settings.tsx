import React from "react";
import {
  Flex,
  IconButton,
  Dialog,
  Text,
  TextField,
  Button,
  // DialogRoot,
  // DialogTrigger,
  // DialogContent,
  // DialogTitle,
  // DialogDescription,
  // DialogClose,
} from "@radix-ui/themes";
import { GearIcon } from "@radix-ui/react-icons";
import { useApiKey } from "../../hooks";

export const Settings: React.FC<{
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
}> = ({ onClick }) => {
  const [apiKey, setApiKey] = useApiKey();

  return (
    <Flex p="4">
      <Dialog.Root>
        <Dialog.Trigger>
          <IconButton variant="outline" onClick={onClick}>
            <GearIcon />
          </IconButton>
        </Dialog.Trigger>

        <Dialog.Content>
          <Dialog.Title>Settings</Dialog.Title>
          <Dialog.Description>Change chat settings</Dialog.Description>
          <Flex asChild>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                const data = new FormData(event.currentTarget);
                const key = data.get("apiKey");
                if (key && typeof key === "string") {
                  setApiKey(key);
                }
              }}
            >
              <label style={{ width: "100%" }}>
                <Text as="div" size="2" mb="1" weight="bold">
                  API Key
                </Text>
                <TextField.Input
                  name="apiKey"
                  type="text"
                  defaultValue={apiKey}
                  placeholder="Enter your refact api key"
                />
              </label>
            </form>
          </Flex>
          <Flex gap="3" mt="4" justify="end">
            <Dialog.Close>
              <Button variant="soft" color="gray">
                Cancel
              </Button>
            </Dialog.Close>
            <Dialog.Close>
              <Button type="submit">Save</Button>
            </Dialog.Close>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>
    </Flex>
  );
};
