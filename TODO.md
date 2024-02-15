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

[x] rag commands come from the caps url.

[X] ensure vscode api is called only once
[x] vscode specific commands and components
[x] export the types for re-use in refact-vscode
[x] vscode attach file
[x] send some info to the chat about the current file open in vscode, probably a REQUEST and RECEIVE file info events
[x] new file button
[x] paste diff button

[ ] check what happens when the lsp isn't on in vscode
[ ] in vscode attach shouldn't show if there's no files (like when opening the ide)
[ ] canceling chat doesn't seems to work (the spinner keeps spinning) :/
[x] build the events (+ types) as a dedicated file
[ ] automate publishing the main branch
[x] export the chat history component
[x] add vscode specific button for opening the history in a tab
[ ] should be monotype font on tooltip (will require adding a custom tooltip)

[x] command completion combobox interactions
[ ] maybe add optimistic cache for queries to lsp?
[x] remove context latest context files from chat when sending a message
[x] small but in command deletion, type @fi tab delete delete then tab
[x] workspace being run twice ? or adding extra files
[x] update readme with the new features/options
[x] uninstall react-cookie and delete code comments
[x] fix flaky test for multiple commands
[x] figure out why the combobox is sometimes not removing the input trigger
[x] add temp file storage when the user uses @ commands
[x] set the model when using @ commands
[x] prevent the user from changing the model when there are temp files
[x] add new line after command
[x] add flex grow to history list
[x] save last used model
[x] increase textarea height with user input
[x] send whole user input when previewing a command
[x] replace file preview when receiving command preview
[x] don't add a new line if command is executable but has no arguments
[x] use syntax highlighting in the users message
[x] bug when running retry, user message isn't removed
[x] bug with the combobox being open after asking a question
[ ] file preview should scroll with textarea
[ ] chat content should stay at end when textarea grows

### EVENTS TODO FOR IDEs

[x] add missing events
[x] open new file
[x] diff paste back
[x] open chat in new tab
[x] send chat to side bar
[x] stop streaming button
[x] error handling (done)
[x] back from chat (when in side-bar)
[x] open chat in new tab (side bar only)
[x] send chat to side bar
[x] create lib directory for code that becomes a lib
[x] configure vite to output multiple entry files (one for web and one for the ide's)
[x] export events in package.json or from lib
[ ] remove inline styles?
[x] vscode select text, click new chat the selected code should be in the chat
