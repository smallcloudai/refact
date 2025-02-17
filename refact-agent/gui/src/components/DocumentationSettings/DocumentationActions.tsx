import {
  Button,
  Dialog,
  DropdownMenu,
  Flex,
  Text,
  TextField,
} from "@radix-ui/themes";
import { DocumentationSource } from "./DocumentationSettings";
import { useState } from "react";

type DocumentationActionsProps = {
  source: DocumentationSource;
  deleteDocumentation: (url: string) => void;
  editDocumentation: (url: string, maxDepth: number, maxPages: number) => void;
  refetchDocumentation: (url: string) => void;
};

export const DocumentationActions: React.FC<DocumentationActionsProps> = ({
  source,
  deleteDocumentation,
  editDocumentation,
  refetchDocumentation,
}: DocumentationActionsProps) => {
  const [maxDepth, setMaxDepth] = useState(source.maxDepth);
  const [maxPages, setMaxPages] = useState(source.maxPages);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  return (
    <>
      <DropdownMenu.Root onOpenChange={setIsDropdownOpen}>
        <DropdownMenu.Trigger>
          <Button variant="soft">
            Actions
            <DropdownMenu.TriggerIcon />
          </Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content>
          <DropdownMenu.Item
            onSelect={() => {
              setIsDialogOpen(true);
            }}
          >
            Edit
          </DropdownMenu.Item>
          <DropdownMenu.Item onSelect={() => refetchDocumentation(source.url)}>
            Refetch
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item
            color="red"
            onClick={() => deleteDocumentation(source.url)}
          >
            Delete
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <Dialog.Root
        open={isDialogOpen && !isDropdownOpen}
        onOpenChange={setIsDialogOpen}
      >
        <Dialog.Content maxWidth="450px">
          <Dialog.Title>{`Edit ${source.url}`}</Dialog.Title>
          <Flex direction="column" gap="3">
            {/**TODO: text can be have `as="label"` and wrap the TextField  */}
            <label>
              <Text as="div" size="2" mb="1" weight="bold">
                Max depth
              </Text>
              <TextField.Root
                defaultValue={maxDepth}
                onChange={(change) => setMaxDepth(Number(change.target.value))}
                type="number"
                placeholder="Enter the max depth"
              />
            </label>
            <label>
              <Text as="div" size="2" mb="1" weight="bold">
                Max pages
              </Text>
              <TextField.Root
                defaultValue={maxPages}
                onChange={(change) => setMaxPages(Number(change.target.value))}
                type="number"
                placeholder="Enter the max pages"
              />
            </label>
          </Flex>

          <Flex gap="3" mt="4" justify="end">
            <Dialog.Close>
              <Button
                variant="soft"
                color="gray"
                onClick={() => {
                  setMaxDepth(source.maxDepth);
                  setMaxPages(source.maxPages);
                }}
              >
                Cancel
              </Button>
            </Dialog.Close>
            <Dialog.Close>
              <Button
                onClick={() => {
                  editDocumentation(source.url, maxDepth, maxPages);
                }}
              >
                Save
              </Button>
            </Dialog.Close>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>
    </>
  );
};
