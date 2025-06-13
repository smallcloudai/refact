import { Box, TextField, TextArea, Text, Switch } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { useCallback, useEffect, useRef, useState } from "react";

// Custom Input Field
export const CustomInputField = ({
  value,
  placeholder,
  type,
  id,
  name,
  size = "long",
  color = "gray",
  onChange,
  wasInteracted = false,
}: {
  id?: string;
  wasInteracted?: boolean;
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
  placeholder?: string;
  size?: string;
  width?: string;
  color?: TextField.RootProps["color"];
  onChange?: (value: string) => void;
}) => {
  const wasInitialized = useRef(wasInteracted);
  // a little hacky, but in this way we avoid of use of formData
  useEffect(() => {
    if (!wasInitialized.current && onChange) {
      onChange(value ?? "");
      wasInitialized.current = true;
    }
  }, [onChange, value]);

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
          placeholder={placeholder}
          onChange={(e) => onChange?.(e.target.value)}
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
  value,
  onChange,
}: {
  id: string;
  name: string;
  value: boolean;
  onChange: (value: boolean) => void;
}) => {
  const [checked, setChecked] = useState(value);

  const onCheckedChange = useCallback(
    (value: boolean) => {
      setChecked(value);
      onChange(value);
    },
    [onChange],
  );

  return (
    <Box>
      <Switch
        name={name}
        id={id}
        size="2"
        checked={checked}
        defaultChecked={value}
        onCheckedChange={onCheckedChange}
      />
      <input type="hidden" name={name} value={checked ? "on" : "off"} />
    </Box>
  );
};
