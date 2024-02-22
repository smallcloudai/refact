import React, { useEffect } from "react";
import {
  Flex,
  IconButton,
  Dialog,
  Text,
  TextField,
  Button,
} from "@radix-ui/themes";
import { GearIcon } from "@radix-ui/react-icons";
import { useApiKey } from "../../hooks";

export const Settings: React.FC = () => {
  const [apiKey, setApiKey] = useApiKey();
  const [keyValue, setValue] = React.useState(apiKey);
  const [open, setOpen] = React.useState(false);

  useEffect(() => {
    setValue(apiKey);
  }, [apiKey, open]);

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setApiKey(keyValue);
    setOpen(false);
  };

  return (
    <Flex p="4">
      <Dialog.Root open={open} onOpenChange={setOpen}>
        <Dialog.Trigger>
          <IconButton variant="outline" title="Settings">
            <GearIcon />
          </IconButton>
        </Dialog.Trigger>

        <Dialog.Content onOpenAutoFocus={(event) => event.preventDefault()}>
          <Dialog.Title>Settings</Dialog.Title>
          <Dialog.Description>Change chat settings</Dialog.Description>
          <Flex asChild pt="4">
            <form onSubmit={handleSubmit}>
              <label style={{ width: "100%" }}>
                <Text as="div" size="2" mb="1" weight="bold">
                  API Key
                </Text>
                <TextField.Input
                  name="apiKey"
                  type="text"
                  value={keyValue}
                  onChange={(event) => setValue(event.target.value)}
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
              <Button onClick={() => setApiKey(keyValue)} type="submit">
                Save
              </Button>
            </Dialog.Close>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>
    </Flex>
  );
};
