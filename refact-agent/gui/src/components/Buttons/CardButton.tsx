import React from "react";
import { Card } from "@radix-ui/themes";

export type CardButtonProps = React.JSX.IntrinsicElements["button"];

export const CardButton: React.FC<CardButtonProps> = (props) => {
  return (
    <Card
      style={{
        width: "100%",
        marginBottom: "2px",
        opacity: props.disabled ? 0.8 : 1,
      }}
      variant="surface"
      className="rt-Button"
      asChild
      role="button"
    >
      <button {...props} />
    </Card>
  );
};
