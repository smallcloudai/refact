import { ArrowDownIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";

type ScrollToBottomButtonProps = {
  onClick: () => void;
};

export const ScrollToBottomButton = ({
  onClick,
}: ScrollToBottomButtonProps) => {
  return (
    <IconButton
      style={{
        position: "absolute",
        width: 50,
        height: 50,
        bottom: 20,
        right: 20,
      }}
      onClick={onClick}
    >
      <ArrowDownIcon width={25} height={25} />
    </IconButton>
  );
};
