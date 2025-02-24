import React, { useCallback, useEffect } from "react";
import {
  Card,
  Flex,
  Text,
  Button,
  TextField,
  TextArea,
  TextAreaProps,
  Heading,
} from "@radix-ui/themes";
import {
  isAddMemoryRequest,
  isMemUpdateRequest,
  knowledgeApi,
  MemoRecord,
  MemUpdateRequest,
} from "../../services/refact";
import styles from "./KnowledgeForms.module.css";

type EditKnowledgeFormProps = {
  memory: MemoRecord;
  onClose: () => void;
};
// TODO: this is similar to Add memory.
export const EditKnowledgeForm: React.FC<EditKnowledgeFormProps> = ({
  memory,
  onClose,
}) => {
  const [submit, result] = knowledgeApi.useUpdateMemoryMutation();

  const handleSubmit = useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const formData = Object.fromEntries(new FormData(event.currentTarget));
      const oldData: MemUpdateRequest = {
        memid: memory.memid,
        mem_type: memory.m_type,
        goal: memory.m_goal,
        project: memory.m_goal,
        payload: memory.m_payload,
        origin: memory.m_origin,
      };
      const updatedMemory = { ...oldData, ...formData };
      // TODO: handle errors
      if (isMemUpdateRequest(updatedMemory)) {
        void submit(updatedMemory);
      }
    },
    [memory, submit],
  );

  useEffect(() => {
    if (result.isSuccess) {
      onClose();
    }
  }, [onClose, result.isSuccess]);

  return (
    <Card asChild>
      <form onSubmit={handleSubmit} onReset={onClose}>
        <FormTitle>Edit a memory</FormTitle>
        <Flex gap="8" direction="column">
          <Flex direction="column" gap="3">
            <TextInput name="goal" label="Goal" defaultValue={memory.m_goal} />
            <TextInput
              name="project"
              label="Project"
              defaultValue={memory.m_project}
            />
            <TextAreaInput
              name="payload"
              label="Payload"
              defaultValue={memory.m_payload}
            />
          </Flex>

          <Flex gap="3" justify="end">
            <Button type="submit" color="green">
              Save
            </Button>
            <Button variant="soft" color="gray" type="reset">
              Close
            </Button>
          </Flex>
        </Flex>
      </form>
    </Card>
  );
};

// TODO: for adding, will change slightly
export const AddKnowledgeForm: React.FC<{ onClose: () => void }> = ({
  onClose,
}) => {
  const [submit, result] = knowledgeApi.useAddMemoryMutation();

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const formData = new FormData(event.currentTarget);
    const memory = Object.fromEntries(formData.entries());

    if (isAddMemoryRequest(memory)) {
      // TODO: handle errors
      submit(memory)
        .unwrap()
        .then(() => {
          event.currentTarget.reset();
        })
        .catch(() => ({}));
    }
  };

  useEffect(() => {
    if (result.isSuccess) {
      onClose();
    }
  }, [result.isSuccess, onClose]);

  return (
    <Card asChild className={styles.knowledge__form}>
      <form onSubmit={handleSubmit} onReset={onClose}>
        <FormTitle>Add a new memory</FormTitle>

        <Flex gap="8" direction="column">
          <Flex direction="column" gap="4">
            <TextInput name="goal" label="Goal" required />
            <TextAreaInput name="payload" label="Payload" required />
          </Flex>

          <Flex gap="3" justify="end">
            <Button type="submit" color="green">
              Save
            </Button>
            <Button variant="soft" color="gray" type="reset">
              Close
            </Button>
          </Flex>
        </Flex>
      </form>
    </Card>
  );
};

const TextInput: React.FC<TextField.RootProps & { label: React.ReactNode }> = ({
  label,
  ...props
}) => {
  return (
    <Text as="label" htmlFor={props.name}>
      {label}
      <TextField.Root {...props} />
    </Text>
  );
};

const TextAreaInput: React.FC<TextAreaProps & { label: React.ReactNode }> = ({
  label,
  ...props
}) => {
  return (
    <Text as="label" htmlFor={props.name}>
      {label}
      <TextArea {...props} />
    </Text>
  );
};

const FormTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <Flex justify="center">
      <Heading as="h4" mb="4" size="4">
        {children}
      </Heading>
    </Flex>
  );
};
