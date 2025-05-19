import { Button, Flex, Heading } from "@radix-ui/themes";
import { SmartLink } from "../../SmartLink";
import { FC } from "react";
import { Integration, SmartLink as TSmartLink } from "../../../services/refact";
import { useAppSelector } from "../../../hooks";
import { useEventsBusForIDE } from "../../../hooks/useEventBusForIDE";
import { selectConfig } from "../../../features/Config/configSlice";

type FormSmartlinksProps = {
  integration: Integration;
  smartlinks: TSmartLink[] | undefined;
  availabilityValues: Record<string, boolean>;
};

export const FormSmartlinks: FC<FormSmartlinksProps> = ({
  smartlinks,
  integration,
  availabilityValues,
}) => {
  const config = useAppSelector(selectConfig);
  const { openFile } = useEventsBusForIDE();

  const { integr_name, project_path, integr_config_path } = integration;

  if (!smartlinks?.length) return null;

  return (
    <Flex width="100%" direction="column" gap="1" mb="6">
      <Flex
        align="baseline"
        direction={{ initial: "column-reverse", xs: "row" }}
        justify="between"
        gap="4"
      >
        <Flex align="center" gap="3" mt="2" justify="start" wrap="wrap">
          <Heading as="h6" size="2" weight="medium">
            Actions:
          </Heading>
          {smartlinks.map((smartlink, idx) => {
            return (
              <SmartLink
                key={`smartlink-${idx}`}
                smartlink={smartlink}
                integrationName={integr_name}
                integrationProject={project_path}
                integrationPath={integr_config_path}
                shouldBeDisabled={
                  smartlink.sl_enable_only_with_tool
                    ? !availabilityValues.on_your_laptop
                    : false
                }
              />
            );
          })}
        </Flex>
        {config.host !== "web" && (
          <Button
            variant="outline"
            color="gray"
            type="button"
            title={`Open ${integr_name}.yaml configuration file in your IDE`}
            onClick={() =>
              openFile({
                file_name: integr_config_path,
                line: 1,
              })
            }
          >
            Open {integr_name}.yaml
          </Button>
        )}
      </Flex>
    </Flex>
  );
};
