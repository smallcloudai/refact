import {ReactElement} from 'react'
import {render, RenderOptions} from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>,
) => {
    // TODO: figure out why this type is wrong :/
    const user = userEvent.setup()
    return {
       ...render(ui, {...options}),
       user,
    }
}

// eslint-disable-next-line react-refresh/only-export-components
export * from '@testing-library/react'

export {customRender as render}