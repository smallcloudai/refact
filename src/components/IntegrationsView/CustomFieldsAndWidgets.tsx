import {
  Box,
  Checkbox,
  TextField,
  TextArea,
  Button,
  Text,
  Switch,
} from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { type ChangeEventHandler, useState } from "react";

// Custom Input Field
export const CustomInputField = ({
  value,
  defaultValue,
  placeholder,
  type,
  id,
  name,
  size = "long",
  color = "gray",
  onChange,
}: {
  id?: string;
  type?:
    | "number"
    | "search"
    | "time"
    | "text"
    | "hidden"
    | "tel"
    | "url"
    | "email"
    | "date"
    | "password"
    | "datetime-local"
    | "month"
    | "week";
  value?: string;
  name?: string;
  defaultValue?: string | number;
  placeholder?: string;
  size?: string;
  width?: string;
  color?: TextField.RootProps["color"];
  onChange?: ChangeEventHandler<HTMLInputElement>;
}) => {
  return (
    <Box width="100%">
      {size !== "multiline" ? (
        <TextField.Root
          id={id}
          name={name}
          type={type}
          size="2"
          value={value}
          variant="soft"
          color={color}
          defaultValue={defaultValue}
          placeholder={placeholder}
          onChange={onChange}
        />
      ) : (
        <TextArea
          id={id}
          name={name}
          size="2"
          rows={3}
          value={value}
          variant="soft"
          color="gray"
          defaultValue={defaultValue?.toString()}
          placeholder={placeholder}
        />
      )}
    </Box>
  );
};

export const CustomLabel = ({
  label,
  htmlFor,
  mt,
}: {
  label: string;
  htmlFor?: string;
  mt?: string;
}) => {
  return (
    <Text
      as="label"
      htmlFor={htmlFor}
      size="2"
      weight="medium"
      mt={mt ? mt : "0"}
      style={{
        display: "block",
      }}
    >
      {label}
    </Text>
  );
};

export const CustomDescriptionField = ({
  children = "",
  mb = "2",
}: {
  children?: string;
  mb?: string;
}) => {
  return (
    <Text
      size="1"
      mb={{
        initial: "0",
        xs: mb,
      }}
      style={{ display: "block", opacity: 0.85 }}
    >
      <Markdown>{children}</Markdown>
    </Text>
  );
};

export const CustomBoolField = ({
  id,
  name,
  defaultValue,
}: {
  id: string;
  name: string;
  defaultValue: boolean;
}) => {
  const [checked, setChecked] = useState(defaultValue);
  return (
    <Box>
      <Switch
        name={name}
        id={id}
        size="2"
        checked={checked}
        defaultChecked={defaultValue}
        onCheckedChange={(value: boolean) => setChecked(value)}
      />
      <input type="hidden" name={name} value={checked ? "on" : "off"} />
    </Box>
  );
};

// Custom Textarea Widget
export const CustomTextareaWidget = () => {
  return (
    <Box>
      <label htmlFor={"d"}>Test label * required</label>
      <TextArea id={"d"} />
    </Box>
  );
};

// Custom Checkbox Widget
export const CustomCheckboxWidget = () => {
  return (
    <Box>
      <label htmlFor={"d"}>
        <Checkbox id={"d"} />
        label * required
      </label>
    </Box>
  );
};
export function AddButton() {
  return (
    <Button size="1" color="green">
      <Text>Add</Text>
    </Button>
  );
}

export function RemoveButton() {
  return (
    <Button size="1" color="ruby" type="button">
      <Text>Remove</Text>
    </Button>
  );
}
export function MoveUpButton() {
  return (
    <Button size="1" color="gray" highContrast type="button">
      <Text>Move Up</Text>
    </Button>
  );
}
export function MoveDownButton() {
  return (
    <Button size="1" color="gray" highContrast type="button">
      <Text>Move Down</Text>
    </Button>
  );
}
