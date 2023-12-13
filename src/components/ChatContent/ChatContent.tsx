import React from 'react'
import { ChatMessages } from '../../services/refact'
import Markdown from 'react-markdown'

export const ChatContent: React.FC<{messages: ChatMessages}> = ({messages}) => {
    return (<div>
        {messages.map(([_role, text], index) => (
            <Markdown key={index}>{text}</Markdown>
        ))}
    </div>)
}