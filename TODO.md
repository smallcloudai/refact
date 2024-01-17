# TODOs

[-] login
[ ] use lsp handlers for chat (keys and config)
[-] no need for logins it's passed when the lsp starts

[x] check cors issues with lsp
[ ] clean events

[?] How should it handle going offline?

### for http release

[?] get config (lsp url)
[x] generic text area
[x] fix scroll area
[x] get caps (models)
[x] handle errors
[x] disable inputs while streaming
[x] remove item from history
[x] code block scroll area (wrap for now)
[ ] type `postMessage` and `dispatch` calls

[?] user markdown input?
[x] enable dark mode

[ ] use grid layout for chat and sidebar?

### PRIORITY

[?] set lsp url
[x] model selection
[x] no api key
[x] Test cases (selecting model, errors, messages)
[x] remove item from history
[x] disable inputs while streaming
[x] stop stream button
[x] build the app (also think about how it'll be configured)
[x] content for when chat is empty
[x] fix the text area placement (empty chat content might help with this)
[x] make it look nice
[x] handle being offline
[x] handle long requests

[x] scroll lags a bit
[x] attach file (this will be different between docker and IDE's)
[x] use the event bus to handle the file upload in the browser this can be done with the file system api using `window.showOpenFilePicker()`
[x] should we allow multiple context files?
[ ] context file display could be an accordion button

[-] confirm if the lsp only responds with assistant deltas

[x] should context file be an array of files?
[x] disable adding a file after a question has been asked
[x] add a global variable style sheet "theme" in self-hosted

[x] add a context to configuration options like vecdb, and host can be added at the top level (this will change the layout and enable/disable features like darkmode, and vecdb)

[x] hard code @commands for now but it the future they will be fetched
[x] combobox for the @commands
[x] add combobox to chat form and maybe pass text-area as a child component
[x] remove test commands

[ ] rag commands come from the caps url.

[ ] ensure vscode api is called only once

### EVENTS TODO FOR IDEs

[ ] add missing events
[ ] open new file
[ ] diff paste back
[ ] open chat in new tab
[ ] send chat to side bar
[x] stop streaming button
[x] error handling (done)
[ ] back from chat (when in side-bar)
[ ] open chat in new tab (side bar only)
[ ] send chat to side bar
[x] create lib directory for code that becomes a lib
[ ] configure vite to output multiple entry files (one for web and one for the ide's)
[x] export events in package.json or from lib
[ ] remove inline styles
