import React from "react";
import classNames from "classnames";
import { Box } from "@radix-ui/themes";
import styles from "./ChatForm.module.css";

export const Form: React.FC<
  React.PropsWithChildren<{
    className?: string;
    onSubmit: React.FormEventHandler<HTMLFormElement>;
    disabled?: boolean;
  }>
> = ({ className, onSubmit, ...props }) => {
  return (
    <Box mt="1">
      <form
        className={classNames(styles.chatForm, className)}
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit(event);
        }}
        {...props}
      />
    </Box>
  );
};
