import { Flex, Heading } from "@radix-ui/themes";
import { SmartLink } from "../../SmartLink";
import { FC } from "react";
import { Integration, SmartLink as TSmartLink } from "../../../services/refact";

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
  if (!smartlinks?.length) return null;

  return (
    <Flex width="100%" direction="column" gap="1" mb="6">
      <Flex align="center" gap="3" mt="2" wrap="wrap">
        <Heading as="h6" size="2" weight="medium">
          Actions:
        </Heading>
        {smartlinks.map((smartlink, idx) => {
          const { integr_name, project_path, integr_config_path } = integration;
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
    </Flex>
  );
};
