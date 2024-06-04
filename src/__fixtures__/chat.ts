import {
  AssistantMessage,
  ChatMessage,
  ChatMessages,
  ChatThread,
  ToolCall,
  ToolMessage,
  ToolResult,
} from "../events";

export const MARS_ROVER_CHAT: ChatThread = {
  id: "9afd6fef-3e49-40df-8aca-688af3621514",
  messages: [
    // [
    //   "system",
    //   "Use backquotes for code blocks. Pay close attention to indent when editing code blocks: indent must be exactly the same as in the original code block.",
    // ],
    [
      "context_memory",
      [
        {
          memo_id: "note20240603_call_U54RGiVERn2zDEIpXuNIHLGy.txt",
          memo_text: "Number of correction points 1\n",
        },
        {
          memo_id: "note20240603_call_E1u9kD6fvb23q56QecnwC6QP.txt",
          memo_text:
            "Failed to retrieve the content of the files when attempting to explain the different properties of Frog and Toad.\n",
        },
        {
          memo_id: "note20240603_call_t8R0pFerpCcvdtTkRwiw2e6S.txt",
          memo_text:
            "Repeated the same unsuccessful attempt to fetch the content of the files.\n",
        },
      ],
    ],
    ["user", "mars rover kata"],
    [
      "context_file",
      [
        {
          file_name: "file/name.txt",
          file_content: "hello",
          line1: 1,
          line2: 2,
          usefulness: 100,
        },
      ],
    ],
    [
      "assistant",
      "The Mars Rover Kata is a coding exercise that simulates the movement of a rover on the surface of Mars. The goal is to write a program that takes in a series of commands and outputs the final position of the rover.\n\nHere is a link to the official Mars Rover Kata documentation: https://kata-log.rocks/mars-rover-kata\n\nYou can find various implementations and solutions to the Mars Rover Kata on platforms like GitHub or coding challenge websites.",
    ],
    ["user", "solve in ocaml"],
    [
      "assistant",
      "Certainly! Here's an example solution to the Mars Rover Kata in OCaml:\n\n```ocaml\ntype direction = North | East | South | West\n\ntype rover = {\n  x : int;\n  y : int;\n  dir : direction;\n}\n\nlet move_forward rover =\n  match rover.dir with\n  | North -> { rover with y = rover.y + 1 }\n  | East -> { rover with x = rover.x + 1 }\n  | South -> { rover with y = rover.y - 1 }\n  | West -> { rover with x = rover.x - 1 }\n\nlet turn_left rover =\n  match rover.dir with\n  | North -> { rover with dir = West }\n  | East -> { rover with dir = North }\n  | South -> { rover with dir = East }\n  | West -> { rover with dir = South }\n\nlet turn_right rover =\n  match rover.dir with\n  | North -> { rover with dir = East }\n  | East -> { rover with dir = South }\n  | South -> { rover with dir = West }\n  | West -> { rover with dir = North }\n\nlet execute_command rover command =\n  match command with\n  | 'F' -> move_forward rover\n  | 'L' -> turn_left rover\n  | 'R' -> turn_right rover\n  | _ -> rover (* Ignore unknown commands *)\n\nlet rec execute_commands rover commands =\n  match commands with\n  | [] -> rover\n  | command :: rest ->\n    let new_rover = execute_command rover command in\n    execute_commands new_rover rest\n\nlet initial_rover = { x = 0; y = 0; dir = North }\n\nlet final_rover = execute_commands initial_rover ['F'; 'R'; 'F'; 'F'; 'L'; 'F']\n\nlet () =\n  Printf.printf \"Final position: (%d, %d), facing %s\\n\"\n    final_rover.x final_rover.y\n    (match final_rover.dir with\n    | North -> \"North\"\n    | East -> \"East\"\n    | South -> \"South\"\n    | West -> \"West\")\n```\n\nThis solution defines the necessary types, functions, and commands to simulate the movement of the rover. The `execute_commands` function takes in a rover and a list of commands and iteratively executes each command to update the rover's position and direction. Finally, the `final_rover` is printed to the console.\n\nYou can run this code in an OCaml environment or compile it using an OCaml compiler.",
    ],

    // remark rehype-katex works with this
    ["user", "the quadratic formula."],
    [
      "assistant",
      // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
      // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
      "The quadratic formula is given by:$$[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} ] $$You can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
    ],
    ["user", "formalla with new lines"],
    [
      "assistant",
      // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
      // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
      "The quadratic formula is given by:\n$$\nx = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}}\n$$\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
    ],
    ["user", "other math"],
    [
      "assistant",
      // "$$ \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a} $$",
      // "The quadratic formula is given by:\n\n\\[ x = \\frac{{-b \\pm \\sqrt{{b^2 - 4ac}}}}{{2a}} \\]\n\nYou can find more information about the quadratic formula in the following link: [Quadratic Formula - Wikipedia](https://en.wikipedia.org/wiki/Quadratic_formula)",
      "block\n\n```math\nC_L\n```\n\ninline: $C_L$\n\n",
    ],
  ],
  title: "mars rover kata",
  model: "gpt-3.5-turbo",
  //   createdAt: "2023-12-21T17:32:50.186Z",
  //   lastUpdated: "2023-12-21T17:33:22.209Z",
};

const chatWithFunctionsRaw = [
  {
    role: "system",
    content:
      '\nYou are a search agent. You need to actively search for the answer yourself, don\'t ask the user to do anything. The answer is most likely in the files and databases accessible using tool calls, not on the internet.\n\nWhen responding to a query, first provide a very brief explanation of your plan to use tools in parallel to answer the question, and then make several tool calls to gather more details.\n\nMinimize the number of steps, call up to 15 tools in parallel when exploring.\n\nIT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST!\n\nWhen user corrects you, acknowledge the correction, write "I will make a note to remember this" and use note_to_self call.\n\n\nExample 1\n\nUser: "What is the weather like today in Paris and London?"\nAssistant: "Must be sunny in Paris and foggy in London."\nUser: "don\'t hallucinate, use the tools"\nAssistant: "Sorry for the confusion, you are right, weather is real-time, and my best shot is to use the weather tool. I will make a note to remember this. My original instruction was to fetch weather for Paris and London. I will use 2 calls in parallel."\n[Call note_to_self "Weather" "For weather, use tools to get real-time information"]\n[Call weather "London"]\n[Call weather "Paris"]\n\n\nExample 2\n\nUser: "What is MyClass"\nAssistant: "Let me find it first."\n[Call ls "."]\nTool: subdir1, subdir2, subdir3\nAssistant: "I see 3 subdirs, will make 3 calls in parallel to check what\'s inside."\n[Call ls "subdir1"]\n[Call ls "subdir2"]\n[Call ls "subdir3"]\nTool: ...\nTool: ...\nTool: ...\nAssistant: "I give up, I can\'t find a file relevant for MyClass \ud83d\ude15"\nUser: "Look, it\'s my_class.cpp"\nAssistant: "Sorry for the confusion, there is in fact a file named `my_class.cpp` in `subdir2` that must be relevant for MyClass. I will make a note to remember this. My original instruction was to describe MyClass."\n[Call note_to_self "searching, browsing filesystem, MyClass" "Try to lowercase the name user asked about, convert to snake case, and guess the right file this way."]\n[Call cat "subdir2/my_class.cpp"]\nTool: ...\nAssistant: "MyClass does this and this"\n\n\n',
    tool_calls: null,
    finish_reason: "",
    tool_call_id: "",
  },
  {
    role: "context_memory",
    content: [
      {
        memo_id: "note20240603_call_U54RGiVERn2zDEIpXuNIHLGy.txt",
        memo_text: "Number of correction points 1\n",
      },
      {
        memo_id: "note20240603_call_E1u9kD6fvb23q56QecnwC6QP.txt",
        memo_text:
          "Failed to retrieve the content of the files when attempting to explain the different properties of Frog and Toad.\n",
      },
      {
        memo_id: "note20240603_call_t8R0pFerpCcvdtTkRwiw2e6S.txt",
        memo_text:
          "Repeated the same unsuccessful attempt to fetch the content of the files.\n",
      },
    ],
  },
  {
    role: "user",
    content: "Explain what Frog is",
    tool_calls: null,
    finish_reason: "",
    tool_call_id: "",
  },
  {
    role: "assistant",
    content:
      "Let me find the relevant information about Frog first. I will check the contents of files or directories that might contain information about Frog. I will start by listing the files in the current directory to see if there are any relevant files or directories.",
    tool_calls: [
      {
        id: "call_WOyQ1sykVGppzWjjUu1drk6L",
        function: {
          arguments: '{"dirpath":"."}',
          name: "ls",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "Listing directory .\n      2260    file Cargo.toml\n      1530    file LICENSE\n       224     dir target\n      1198    file mycaps_te3.json\n       416     dir tests\n    152298    file Cargo.lock\n       757    file mycaps_openai.json\n        61    file build.rs\n      1264    file mycaps_gte.json\n      1598    file _video\n      3548    file README.md\n       768     dir examples\n       219    file _backtrace\n      1665    file _video2\n       141    file a.sh\n       139    file _help\n       992     dir src\n",
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_WOyQ1sykVGppzWjjUu1drk6L",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_IYK970zyp9vZ36m7emzmNDC9",
        function: {
          arguments: '{"filepath": "README.md"}',
          name: "cat",
        },
        type: "function" as const,
      },
      {
        id: "call_8jTn7oj8tfctEnqgKQRBJH0w",
        function: {
          arguments: '{"filepath": "Cargo.toml"}',
          name: "cat",
        },
        type: "function" as const,
      },
      {
        id: "call_Ql7xrkn5BqtjVSHHAnNksFis",
        function: {
          arguments: '{"filepath": "Cargo.lock"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      'File README.md:50-99\n```    "temperature": 0.1,\n    "max_new_tokens": 20\n  }\n}\'\n```\n\nOutput is `[{"code_completion": "\\n    return \\"Hello World!\\"\\n"}]`.\n\n[LSP example](examples/lsp_completion.py)\n\n\n## Telemetry\n\nThe flags `--basic-telemetry` and `--snippet-telemetry` control what telemetry is sent. To be clear: without\nthese flags, no telemetry is sent. Those flags are typically controlled from IDE plugin settings.\n\nBasic telemetry means counters and error messages without information about you or your code. It is "compressed"\ninto `.cache/refact/telemetry/compressed` folder, then from time to time it\'s sent and moved\nto `.cache/refact/telemetry/sent` folder.\n\n"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary\ncommunicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how\nyou see it in `.cache/refact/telemetry`. The files are human-readable.\n\nWhen using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.\n\n\n## Caps File\n\nThe `--address-url` parameter controls the behavior of this program by a lot. The address is first used\nto construct `$URL/coding_assistant_caps.json` address to fetch the caps file. Furthermore, there are\ncompiled-in caps you can use by magic addresses "Refact" and "HF".\n\nThe caps file describes which models are running, default models for completion and chat,\nwhere to send the telemetry, how to download a\ntokenizer, where is the endpoint to access actual language models. To read more, check out\ncompiled-in caps in [caps.rs](src/caps.rs).\n\n\n## Tests\n\nThe one to run often is [test_edge_cases.py](tests/test_edge_cases.py).\n\nYou can also run [measure_humaneval_fim.py](tests/measure_humaneval_fim.py) for your favorite model.\n\n\n## Credits\n\nThe initial version of this project was written by looking at llm-ls by [@McPatate](https://github.com/McPatate). He\'s a Rust fan who inspired this project!\n```',
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_IYK970zyp9vZ36m7emzmNDC9",
  },
  {
    role: "tool",
    content:
      'File Cargo.toml:39-88\n```futures-util = "0.3"\nasync-stream = "0.3.5"\nchrono = "0.4.31"\nregex = "1.9.5"\nasync-trait = "0.1.73"\nsimilar = "2.3.0"\naxum = "0.6.20"\nuuid = { version = "1", features = ["v4"] }\nlazy_static = "1.4.0"\n\nregex-automata = { version = "0.1.10", features = ["transducer"] }\nsorted-vec = "0.8.3"\ntree-sitter = "0.20"\ntree-sitter-cpp = "0.20"\n#tree-sitter-c-sharp = "0.20"\ntree-sitter-java = "0.20"\ntree-sitter-javascript = "0.20"\n#tree-sitter-kotlin = "0.3.1"\ntree-sitter-python = "0.20"\ntree-sitter-rust = "0.20"\ntree-sitter-typescript = "0.20"\n\narrow = "47.0.0"\narrow-array = "47.0.0"\narrow-schema= "47.0.0"\nasync_once= "0.2.6"\nasync-process = "2.0.1"\nitertools = "0.11.0"\nlance = "=0.9.0"\nlance-linalg = "=0.9.0"\nlance-index = "=0.9.0"\nlog = "0.4.20"\nmd5 = "0.7"\nmockito = "0.28.0"\nnotify = { version = "6.1.1", features = ["serde"] }\nparking_lot = { version = "0.12.1", features = ["serde"] }\nrusqlite = { version = "0.30.0", features = ["bundled"] }\ntempfile = "3.8.1"\ntime = "0.3.30"\ntokio-rusqlite = "0.5.0"\nvectordb = "=0.4.0"\nwalkdir = "2.3"\nwhich = "5.0.0"\nstrsim = "0.8.0"\ntypetag = "0.2"\ndyn_partial_eq = "=0.1.2"\nrayon = "1.8.0"\nbacktrace = "0.3.71"\nrand = "0.8.5"\n```',
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_8jTn7oj8tfctEnqgKQRBJH0w",
  },
  {
    role: "tool",
    content:
      'File Cargo.lock:6265-6314\n```]\n\n[[package]]\nname = "zstd"\nversion = "0.11.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "20cc960326ece64f010d2d2107537f26dc589a6573a316bd5b1dba685fa5fde4"\ndependencies = [\n "zstd-safe 5.0.2+zstd.1.5.2",\n]\n\n[[package]]\nname = "zstd"\nversion = "0.12.4"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1a27595e173641171fc74a1232b7b1c7a7cb6e18222c11e9dfb9888fa424c53c"\ndependencies = [\n "zstd-safe 6.0.6",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "5.0.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1d2a5585e04f9eea4b2a3d1eca508c4dee9592a89ef6f450c11719da0726f4db"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "6.0.6"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "ee98ffd0b48ee95e6c5168188e44a54550b1564d9d530ee21d5f0eaed1069581"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-sys"\nversion = "2.0.9+zstd.1.5.5"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "9e16efa8a874a0481a574084d34cc26fdb3b99627480f785888deb6386506656"\ndependencies = [\n "cc",\n "pkg-config",\n]\n```',
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_Ql7xrkn5BqtjVSHHAnNksFis",
  },
  {
    role: "assistant",
    content:
      "Based on the information found in the files:\n\n- Frog seems to be a project related to coding assistance and telemetry.\n- Telemetry is sent based on flags like `--basic-telemetry` and `--snippet-telemetry`.\n- The `Caps File` contains information about models, default models for completion and chat, telemetry destinations, tokenizers, and language models.\n- Tests related to the project include `test_edge_cases.py` and `measure_humaneval_fim.py`.\n- The project credits [@McPatate](https://github.com/McPatate) for the initial version, who is a Rust fan.\n\nFor more specific details about Frog, further exploration into the project's source code or documentation may be needed.",
    tool_calls: null,
    finish_reason: "stop",
    tool_call_id: "",
  },
  {
    role: "user",
    content: "continue searching for the Frog in tests",
    tool_calls: null,
    finish_reason: "",
    tool_call_id: "",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_KKgK2Ki3bpvCk2jCtDhQxfpw",
        function: {
          arguments: '{"dirpath":"tests"}',
          name: "ls",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "Listing directory tests\n      2438    file test_hf_endpoint.py\n      3021    file lsp_connect.py\n       678    file lsp_completion.py\n         0    file __init__.py\n        96     dir __pycache__/\n      1340    file test_at_completion.py\n      3926    file post_doc_info.py\n       224     dir emergency_frog_situation/\n      4107    file test_edge_cases.py\n      2081    file code_completion_with_rag.py\n      2866    file measure_humaneval_fim.py\n",
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_KKgK2Ki3bpvCk2jCtDhQxfpw",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_dT0OdIL7JLYJ7Fxk72MvmuRA",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation/"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
    tool_calls: null,
    finish_reason: "call_failed",
    tool_call_id: "call_dT0OdIL7JLYJ7Fxk72MvmuRA",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_C5uTWek5PUKmaTfe7u0TypZL",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
    tool_calls: null,
    finish_reason: "call_failed",
    tool_call_id: "call_C5uTWek5PUKmaTfe7u0TypZL",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_9vYcfrFCiUhy8g6bNn0WJn5p",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation/"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
    tool_calls: null,
    finish_reason: "call_failed",
    tool_call_id: "call_9vYcfrFCiUhy8g6bNn0WJn5p",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_6Cg6UfAvNTgEt96EQrHz16W6",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
    tool_calls: null,
    finish_reason: "call_failed",
    tool_call_id: "call_6Cg6UfAvNTgEt96EQrHz16W6",
  },
  {
    role: "assistant",
    content:
      "I couldn't retrieve specific information about Frog in the tests directory. If there are any other locations or specific files you suspect might contain details about Frog, please let me know.",
    tool_calls: null,
    finish_reason: "stop",
    tool_call_id: "",
  },
  {
    role: "user",
    content: "use ls, don't be stupid",
    tool_calls: null,
    finish_reason: "",
    tool_call_id: "",
  },
  {
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call_UoHvkwbPq6LMAKRM0iblVkSB",
        function: {
          arguments: '{"dirpath":"tests/emergency_frog_situation"}',
          name: "ls",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "Listing directory tests/emergency_frog_situation\n      1516    file jump_to_conclusions.py\n       695    file set_as_avatar.py\n        96     dir __pycache__/\n       777    file frog.py\n       249    file work_day.py\n",
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_UoHvkwbPq6LMAKRM0iblVkSB",
  },
  {
    role: "assistant",
    content:
      "In the `tests/emergency_frog_situation` directory, the file `frog.py` seems to be relevant to Frog. Let's check the content of `frog.py` to find out more about Frog.",
    tool_calls: [
      {
        id: "call_spx7e7LMfw97BmmzojQQf0rO",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation/frog.py"}',
          name: "cat",
        },
        type: "function" as const,
      },
    ],
    finish_reason: "tool_calls",
    tool_call_id: "",
  },
  {
    role: "tool",
    content:
      "File tests/emergency_frog_situation/frog.py:1-29\n```import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n```",
    tool_calls: null,
    finish_reason: "call_worked",
    tool_call_id: "call_spx7e7LMfw97BmmzojQQf0rO",
  },
  {
    role: "assistant",
    content:
      "The file `frog.py` in the `tests/emergency_frog_situation` directory contains a Python class `Frog` with methods related to simulating the movement of a frog in a pond. The class includes methods for initialization, bouncing off banks, and jumping within the pond. The `Frog` class has attributes like position (`x`, `y`) and velocity (`vx`, `vy`), along with methods to handle movement and boundary conditions.\n\nThis information suggests that Frog in this context is related to a simulation or program involving a frog's behavior in a pond.",
    tool_calls: null,
    finish_reason: "stop",
    tool_call_id: "",
  },
];

export const CHAT_FUNCTIONS_MESSAGES: ChatMessages =
  chatWithFunctionsRaw.reduce<ChatMessages>((acc, message) => {
    if (message.role === "assistant" && message.content !== null) {
      const next: ChatMessages = [[message.role, message.content]];

      if (message.tool_calls) {
        const toolCalls: ToolCall[] = message.tool_calls.map(
          (toolCall, index) => {
            return {
              ...toolCall,
              index,
            };
          },
        );
        const msg: AssistantMessage = ["assistant", message.content, toolCalls];
        next.push(msg);
      }

      return [...acc, ...next];
    }

    if (message.role === "system" && message.content !== null) {
      const msg: ChatMessage = [message.role, message.content];
      return [...acc, msg];
    }

    if (message.role === "user" && message.content !== null) {
      return [...acc, [message.role, message.content]];
    }

    if (message.role === "tool" && message.content !== null) {
      const content: ToolResult = {
        tool_call_id: message.tool_call_id,
        content: message.content,
        finish_reason: message.finish_reason,
      };
      const msg: ToolMessage = [message.role, content];

      return [...acc, msg];
    }

    // if(message.role === "context_file"){}

    return acc;
  }, []);
