import React from "react";
import classNames from "classnames";
import styles from "./ChatForm.module.css";

export const Form: React.FC<
  React.PropsWithChildren<{
    className?: string;
    onSubmit: React.FormEventHandler<HTMLFormElement>;
  }>
> = ({ className, onSubmit, ...props }) => {
  return (
    <form
      className={classNames(styles.chatForm, className)}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(event);
      }}
      {...props}
    />
  );
};
