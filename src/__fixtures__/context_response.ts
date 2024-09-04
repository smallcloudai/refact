import fs from "node:fs";
import path from "node:path";

const responseString = fs.readFileSync(
  path.join(__dirname, "context_response.txt"),
  {
    encoding: "utf8",
  },
);

const responseArray = responseString.split("\n\n");

export const responseStream = () => {
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    start(controller) {
      responseArray.forEach((response) => {
        controller.enqueue(encoder.encode(response));
      });
      controller.close();
    },
  });
  return stream;
};
