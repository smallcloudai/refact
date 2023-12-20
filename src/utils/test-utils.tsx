import { ReactElement } from "react";

import { render, RenderOptions } from "@testing-library/react";
import userEvent, { UserEvent } from "@testing-library/user-event";
import { Theme } from "@radix-ui/themes";

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">,
): ReturnType<typeof render> & { user: UserEvent } => {
  const user = userEvent.setup();
  return {
    ...render(ui, { wrapper: Theme, ...options }),
    user,
  };
};

// eslint-disable-next-line react-refresh/only-export-components
export * from "@testing-library/react";

export { customRender as render };
