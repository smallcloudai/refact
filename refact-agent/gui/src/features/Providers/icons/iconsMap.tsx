import { AnthropicIcon } from "./Anthropic";
import { GeminiIcon } from "./Gemini";
import { GroqIcon } from "./Groq";
import { OpenAIIcon } from "./OpenAI";
import { OpenRouterIcon } from "./OpenRouter";
import { RefactIcon } from "./Refact";

export const iconsMap: Record<string, JSX.Element> = {
  Refact: <RefactIcon />,
  openai: <OpenAIIcon />,
  anthropic: <AnthropicIcon />,
  google_gemini: <GeminiIcon />,
  openrouter: <OpenRouterIcon />,
  groq: <GroqIcon />,
};
