import React from "react";
import { ComboboxItem } from "@ariakit/react";
import { Button } from "@radix-ui/themes";
import styles from "./ComboBox.module.css";

export const Item: React.FC<{
  onClick: React.MouseEventHandler<HTMLDivElement>;
  value: string;
  children: React.ReactNode;
}> = ({ children, value, onClick }) => {
  return (
    <Button className={styles.item} variant="ghost" asChild highContrast>
      <ComboboxItem
        value={value}
        onClick={onClick}
        focusOnHover
        clickOnEnter={false}
        className={styles.combobox__item}
      >
        {children}
      </ComboboxItem>
    </Button>
  );
};
