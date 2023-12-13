import { useEffect } from "react";
import { ChatMessages, sendChat } from "../services/refact";


export function useEventBusForHost() {


		useEffect(() => {
			const controller = new AbortController();
			const listener = (event: MessageEvent) => {
				console.log("HOST EVENT");
				console.log(event.data)
				// TODO: validate the events
				// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
				if(!event.data.type) {return;}
				// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
				switch(event.data.type) {
						case "chat_question": {
								// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
								handleSend(event.data.payload as ChatMessages, controller);
								return;
						}
				}
		};
				window.addEventListener("message", listener);
				return () => {
					controller.abort()
					window.removeEventListener("message", listener);
				}
		}, [])


}

function handleSend(messages: ChatMessages, controller: AbortController) {

		sendChat(messages, "gpt-3.5-turbo", controller).then(response => {
			const decoder = new TextDecoder();
			const reader = response.body?.getReader()
			if(!reader) return;
			void reader.read().then(function pump({ done, value }): Promise<void> {
				if (done) {
					// Do something with last chunk of data then exit reader
					return Promise.resolve();
				}

				const streamAsString = decoder.decode(value)

				const deltas = streamAsString.split("\n\n").filter(str => str.length > 0)
				if(deltas.length === 0) return Promise.resolve();

				for(const delta of deltas) {
					if(!delta.startsWith("data: ")) {
						console.log("Unexpected data in streaming buf: " + delta);
						continue;
					}

					const maybeJsonString = delta.substring(6);
					if(maybeJsonString === "[DONE]") {
						return Promise.resolve(); // handle finish
					}

					if(maybeJsonString === "[ERROR]") {
						console.log("Streaming error");
						const errorJson = JSON.parse(maybeJsonString) as Record<string, unknown>;
						console.log(errorJson)
						return Promise.reject(errorJson.detail || "streaming error"); // handle error
					}
					// figure out how to safely parseJson

					const json = JSON.parse(maybeJsonString) as Record<string, unknown>;

					// console.log(json);
					window.postMessage({
						type: "chat",
						payload: json,
					}, "*");
				}

				return reader.read().then(pump);
		})}).catch(console.error);
	}