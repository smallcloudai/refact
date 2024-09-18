import { Button, Dialog, Flex, Table, Text, TextField } from "@radix-ui/themes";
import { DocumentationActions } from "./DocumentationActions";
import { useState } from "react";
import { ChevronLeftIcon } from "@radix-ui/react-icons";

export interface DocumentationSource {
  url: string;
  maxDepth: number;
  maxPages: number;
  pages: number;
}

export type DocumentationSettingsProps = {
  sources: DocumentationSource[];
  addDocumentation: (url: string, maxDepth: number, maxPages: number) => void;
  deleteDocumentation: (url: string) => void;
  refetchDocumentation: (url: string) => void;
  editDocumentation: (url: string, maxDepth: number, maxPages: number) => void;
};

export const DocumentationSettings: React.FC<DocumentationSettingsProps> = ({
  sources,
  addDocumentation,
  deleteDocumentation,
  editDocumentation,
  refetchDocumentation,
}: DocumentationSettingsProps) => {
  const [url, setUrl] = useState("");
  const [maxDepth, setMaxDepth] = useState(2);
  const [maxPages, setMaxPages] = useState(50);

  return (
    <Flex direction="column" gap="2" maxWidth="540px" m="8px">
      <Text size="4">Documentation sources</Text>
      {sources.length > 0 ? (
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeaderCell>Url</Table.ColumnHeaderCell>
              <Table.ColumnHeaderCell>Pages</Table.ColumnHeaderCell>
              <Table.ColumnHeaderCell></Table.ColumnHeaderCell>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {sources.map((source) => {
              return (
                <Table.Row key={source.url}>
                  <Table.RowHeaderCell>{source.url}</Table.RowHeaderCell>
                  <Table.Cell>{source.pages}</Table.Cell>
                  <Table.Cell>
                    <DocumentationActions
                      source={source}
                      deleteDocumentation={deleteDocumentation}
                      editDocumentation={editDocumentation}
                      refetchDocumentation={refetchDocumentation}
                    />
                  </Table.Cell>
                </Table.Row>
              );
            })}
          </Table.Body>
        </Table.Root>
      ) : (
        <Text min-height="200px">
          No documentation has been added yet. Press the add button to add
          documentation that the chat assistent can use.
        </Text>
      )}
      <Flex direction="row">
        <Button variant="outline" mr="auto">
          <ChevronLeftIcon />
          {"< Back"}
        </Button>
        <Dialog.Root>
          <Dialog.Trigger>
            <Button ml="auto" type="submit">
              {"add"}
            </Button>
          </Dialog.Trigger>
          <Dialog.Content maxWidth="450px">
            <Dialog.Title>Add documentation</Dialog.Title>
            <Dialog.Description size="2" mb="4">
              Add a documentation source that the chat assistent can use.
            </Dialog.Description>

            <Flex direction="column" gap="3">
              <label>
                <Text as="div" size="2" mb="1" weight="bold">
                  Url
                </Text>
                <TextField.Root
                  defaultValue={url}
                  onChange={(event) => {
                    setUrl(event.target.value);
                  }}
                  placeholder="Enter the documentation url"
                />
              </label>
              <label>
                <Text as="div" size="2" mb="1" weight="bold">
                  Max depth
                </Text>
                <TextField.Root
                  defaultValue={maxDepth}
                  onChange={(event) => {
                    setMaxDepth(Number(event.target.value));
                  }}
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
                  onChange={(event) => {
                    setMaxPages(Number(event.target.value));
                  }}
                  type="number"
                  placeholder="Enter the max pages"
                />
              </label>
            </Flex>

            <Flex gap="3" mt="4" justify="end">
              <Dialog.Close>
                <Button variant="soft" color="gray">
                  Cancel
                </Button>
              </Dialog.Close>
              <Dialog.Close>
                <Button
                  onClick={() => {
                    addDocumentation(url, maxDepth, maxPages);
                  }}
                >
                  Add
                </Button>
              </Dialog.Close>
            </Flex>
          </Dialog.Content>
        </Dialog.Root>
      </Flex>
    </Flex>
  );
};
