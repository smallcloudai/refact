import { ReactElement } from 'react';

import {render, RenderOptions} from '@testing-library/react'
import userEvent, { UserEvent } from '@testing-library/user-event'

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>,
): ReturnType<typeof render> & { user: UserEvent}  => {
    // TODO: figure out why this type is wrong :/
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-call, @typescript-eslint/no-unsafe-member-access
    const user = userEvent.setup();
    return {
       ...render(ui, {...options}),
       // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
       user,
    }
}

// eslint-disable-next-line react-refresh/only-export-components
export * from '@testing-library/react'

export {customRender as render}