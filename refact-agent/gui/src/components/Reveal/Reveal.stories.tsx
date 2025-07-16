import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Reveal } from ".";
import { Text, Container, Box } from "@radix-ui/themes";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";

const Template: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Container size="1" p="8">
          {children}
        </Container>
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof Reveal> = {
  title: "Reveal",
  component: Reveal,
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
};

export default meta;

export const Primary: StoryObj<typeof Reveal> = {
  args: {
    children: (
      <Box>
        <Text as="p">
          A component that hides it&apos;s content until it&apos;s revealed.{" "}
        </Text>
        <Text>
          Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do
          eiusmod tempor incididunt ut labore et dolore magna aliqua. Porta
          lorem mollis aliquam ut porttitor leo a diam. Leo a diam sollicitudin
          tempor id eu nisl nunc mi. Pellentesque id nibh tortor id aliquet
          lectus proin nibh. Rhoncus dolor purus non enim praesent elementum
          facilisis leo vel. Et netus et malesuada fames ac turpis. Sit amet
          facilisis magna etiam tempor. Odio tempor orci dapibus ultrices in
          iaculis nunc. Eget egestas purus viverra accumsan in nisl nisi. Lectus
          urna duis convallis convallis tellus id interdum velit laoreet. Turpis
          cursus in hac habitasse platea dictumst quisque sagittis purus. Urna
          nec tincidunt praesent semper feugiat nibh sed pulvinar proin. Sit
          amet facilisis magna etiam tempor. Sed euismod nisi porta lorem mollis
          aliquam ut. Enim diam vulputate ut pharetra.
        </Text>
      </Box>
    ),
  },
};
