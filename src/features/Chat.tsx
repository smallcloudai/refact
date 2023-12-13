import React from 'react'
import { ChatForm } from '../components/ChatForm'
import { useEventBusForChat } from '../hooks/useEventBusForChat'
import { ChatContent } from '../components/ChatContent'


export const Chat: React.FC = () => {
    const {state, askQuestion}  = useEventBusForChat()
    return (<div>
        <ChatContent messages={state.messages} />
        <ChatForm onSubmit={(value) => {
            askQuestion(value)
        }} />
    </div>)
}