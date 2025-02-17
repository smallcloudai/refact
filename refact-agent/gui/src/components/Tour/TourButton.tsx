import { Button, Flex } from "@radix-ui/themes";

export type TourButtonProps = {
  title: string;
  onClick: () => void;
};

export function TourButton({ title, onClick }: TourButtonProps) {
  return (
    <Flex
      direction="row"
      style={{
        // TODO: align prop
        alignItems: "center",
      }}
    >
      <Button
        onClick={onClick}
        style={{ backgroundColor: "#E7150D", flex: 1, padding: 10 }}
      >
        {title}
      </Button>
    </Flex>
  );
}
