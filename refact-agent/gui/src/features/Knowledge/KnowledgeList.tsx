import React from "react";
import {
  Card,
  Flex,
  Heading,
  Spinner,
  Text,
  Button,
  TextField,
  IconButton,
} from "@radix-ui/themes";
import {
  TrashIcon,
  Pencil1Icon,
  MagnifyingGlassIcon,
  PlusIcon,
} from "@radix-ui/react-icons";
import { knowledgeApi, MemoRecord } from "../../services/refact";
import { pop } from "../../features/Pages/pagesSlice";
import { useAppDispatch } from "../../hooks";
import { ScrollArea } from "../../components/ScrollArea";
import { VecDBStatusButton } from "./VecdbStatus";
import { EditKnowledgeForm, AddKnowledgeForm } from "./KnowledgeForms";
import { useKnowledgeSearch } from "./useKnowledgeSearch";

export const KnowledgeList: React.FC = () => {
  const { memories, search, vecDbStatus, isKnowledgeLoaded } =
    useKnowledgeSearch();
  const dispatch = useAppDispatch();

  const [openForm, setOpenForm] = React.useState<boolean>(false);
  const [editing, setEditing] = React.useState<null | string>(null);

  const handleBack = React.useCallback(() => {
    if (openForm) {
      setOpenForm(false);
    } else {
      dispatch(pop());
    }
  }, [dispatch, openForm]);

  const memoryCount = Object.keys(memories).length;

  return (
    <Flex direction="column" overflowY="hidden" height="100%">
      <Flex direction="column" gap="4" mb="4">
        <Flex justify="between">
          <Button variant="outline" onClick={handleBack}>
            Back
          </Button>

          <Flex gap="3">
            <TextField.Root placeholder="Search knowledge" onChange={search}>
              <TextField.Slot>
                <MagnifyingGlassIcon height="16" width="16" />
              </TextField.Slot>
            </TextField.Root>

            <IconButton
              variant="outline"
              title="Add new knowledge"
              disabled={openForm}
              onClick={() => setOpenForm(true)}
            >
              <PlusIcon />
            </IconButton>

            <VecDBStatusButton status={vecDbStatus} />
          </Flex>
        </Flex>

        <Heading ml="auto" mr="auto" as="h4">
          Knowledge
        </Heading>

        {openForm && <AddKnowledgeForm onClose={() => setOpenForm(false)} />}
      </Flex>
      <ScrollArea scrollbars="vertical">
        <Flex direction="column" gap="4" px="2">
          {!isKnowledgeLoaded && <Spinner loading={!isKnowledgeLoaded} />}
          {/* TODO: this could happen if theres no knowledge, but may also happen while waiting for the stream */}
          {isKnowledgeLoaded && memoryCount === 0 && (
            <Text>No knowledge items found</Text>
          )}

          {Object.values(memories).map((memory) => {
            return (
              <KnowledgeListItem
                key={memory.memid}
                memory={memory}
                editing={editing === memory.memid}
                onOpenEdit={() => setEditing(memory.memid)}
                onCloseEdit={() => setEditing(null)}
              />
            );
          })}
        </Flex>
      </ScrollArea>
    </Flex>
  );
};

type KnowledgeListItemProps = {
  memory: MemoRecord;
  editing: boolean;
  onOpenEdit: () => void;
  onCloseEdit: () => void;
};

const KnowledgeListItem: React.FC<KnowledgeListItemProps> = ({
  memory,
  editing,
  onOpenEdit,
  onCloseEdit,
}) => {
  const [deleteMemory, result] = knowledgeApi.useDeleteMemoryMutation();

  const handleDeletion = React.useCallback(() => {
    void deleteMemory(memory.memid);
    // TBD: handle errors
    // TBD: should we clear the form after submit?
    // event.currentTarget.reset();
  }, [deleteMemory, memory.memid]);

  if (editing) {
    return <EditKnowledgeForm memory={memory} onClose={onCloseEdit} />;
  }

  return (
    <Card>
      <Flex direction="column" gap="3">
        <Flex justify="between" align="center">
          <Text size="2" weight="bold">
            {memory.m_goal}
          </Text>
          <Flex gap="2" style={{ alignSelf: "flex-start" }}>
            <IconButton onClick={onOpenEdit} variant="outline">
              <Pencil1Icon />
            </IconButton>

            <IconButton
              onClick={handleDeletion}
              variant="outline"
              loading={result.isLoading}
            >
              <TrashIcon />
            </IconButton>
          </Flex>
        </Flex>

        <Text size="2">{memory.m_payload}</Text>
      </Flex>
    </Card>
  );
};
