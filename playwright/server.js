import http from "node:http";
import path from "node:path";
import fs from "node:fs";

const server = http.createServer((req, res) => {
  if (req.url === "/") {
    res.writeHead(200, { "Content-Type": "text/html" });
    res.end(index);
  }
  if (req.url === "/refact-chat.js") {
    fs.readFile(
      path.join("..", "refact-agent", "gui", "dist", "chat", "index.umd.cjs"),
      (err, data) => {
        if (err) throw err;
        res.writeHead(200, { "Content-Type": "application/javascript" });
        res.end(data);
      }
    );
  }
  if (req.url === "/refact-chat.css") {
    fs.readFile(
      path.join("..", "refact-agent", "gui", "dist", "chat", "style.css"),
      (err, data) => {
        if (err) throw err;
        res.writeHead(200, { "Content-Type": "text/css" });
        res.end(data);
      }
    );
  }
});

server.listen(3000, () => {});

const index = `
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/png" href="/favicon.png" />
    <link rel="stylesheet" href="/refact-chat.css" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1, minimum-scale=1"
    />
    <title>Refact.ai Chat</title>
  </head>
  <body class="vscode-dark">
    <div id="refact-chat"></div>
    <script src="/refact-chat.js"></script>
  </body>
</html>
`;
