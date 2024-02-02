import React from "react";
// import { useCookies } from "react-cookie";
// import { useState } from "react";
import { Flex, Box } from "@radix-ui/themes";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";
// import { useApiKey } from "../../hooks";

export type ChatHistoryProps = {
  history: ChatHistoryItem[];
  onHistoryItemClick: (id: string) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
};

// const useCookieApiKey = () => {
//   const [cookies, setCookie] = useCookies<"api_key", { api_key: string }>([
//     "api_key",
//   ]);

//   const maybeApiKey =
//     cookies.api_key && typeof cookies.api_key === "string"
//       ? cookies.api_key
//       : "";
//   const [value, setValue] = useState<string>(maybeApiKey);

//   useEffect(() => {
//     setValue((cookies.api_key ?? "") as string);
//   }, [cookies.api_key]);

//   return {
//     value,
//     setCookie,
//     setValue,
//   };
// };

// const ApiKeyInput = () => {
//   const [apiKey, setApiKey] = useApiKey();
//   const [value, setValue] = useState<string>(apiKey);
//   const handleBlur = () => {
//     setApiKey(value);
//   };

//   const { value, setCookie, setValue } = useCookieApiKey();
//   const handleBlur = () => {
//     setCookie("api_key", value);
//   };

//   const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
//     setValue(e.target.value);
//   };

//   return (
//     <input
//       type="text"
//       value={value}
//       onChange={handleChange}
//       onBlur={handleBlur}
//       placeholder="Enter API key"
//     />
//   );
// };

export const ChatHistory: React.FC<ChatHistoryProps> = ({
  history,
  onHistoryItemClick,
  onDeleteHistoryItem,
  onOpenChatInTab,
}) => {
  return (
    <Box
      style={{
        overflow: "hidden",
      }}
      pb="2"
    >
      <ScrollArea scrollbars="vertical">
        <Flex justify="center" align="center" pl="2" pr="2" direction="column">
          {history.map((chat) => (
            <HistoryItem
              onClick={onHistoryItemClick}
              onOpenInTab={onOpenChatInTab}
              onDelete={onDeleteHistoryItem}
              key={chat.id}
              chat={chat}
            />
          ))}
        </Flex>
      </ScrollArea>
    </Box>
  );
};
