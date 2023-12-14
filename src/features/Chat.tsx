import React from 'react'
import { ChatForm } from '../components/ChatForm'
import { useEventBusForChat } from '../hooks/useEventBusForChat'
import { ChatContent } from '../components/ChatContent'
import { Flex } from '@radix-ui/themes'

export const Chat: React.FC = () => {
    const {state, askQuestion}  = useEventBusForChat();

    return (<Flex direction="column" justify="between" style={{
        height: "calc(100dvh - 38px)" // TODO: fix this
    }}
    >
        <ChatContent messages={state.messages} />
        <ChatForm onSubmit={(value) => {
            askQuestion(value)
        }} />
    </Flex>)
}