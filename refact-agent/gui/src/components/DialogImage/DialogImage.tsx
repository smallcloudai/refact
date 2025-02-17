import React from "react";
import { Dialog, Avatar, AvatarProps } from "@radix-ui/themes";
import { ImageIcon } from "@radix-ui/react-icons";

export const DialogImage: React.FC<{
  src: string;
  size?: AvatarProps["size"];
  fallback?: AvatarProps["fallback"];
}> = ({ size = "8", fallback = <ImageIcon />, src }) => {
  return (
    <Dialog.Root>
      <Dialog.Trigger>
        <Avatar
          radius="small"
          src={src}
          size={size}
          fallback={fallback}
          style={{ cursor: "zoom-in" }}
        />
      </Dialog.Trigger>
      <Dialog.Content maxWidth="800px">
        <img style={{ objectFit: "cover", width: "100%" }} src={src} />
      </Dialog.Content>
    </Dialog.Root>
  );
};
