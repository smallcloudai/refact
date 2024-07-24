import { ChatMessages, ChatThread } from "../events";
import { ChatState } from "../hooks";

export const MARS_ROVER_CHAT: ChatThread = {
  id: "9afd6fef-3e49-40df-8aca-688af3621514",
  messages: [
    // [
    //   "system",
    //   "Use backquotes for code blocks. Pay close attention to indent when editing code blocks: indent must be exactly the same as in the original code block.",
    // ],
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

export const CHAT_FUNCTIONS_MESSAGES: ChatMessages = [
  [
    "system",
    '\nYou are a search agent. You need to actively search for the answer yourself, don\'t ask the user to do anything. The answer is most likely in the files and databases accessible using tool calls, not on the internet.\n\nWhen responding to a query, first provide a very brief explanation of your plan to use tools in parallel to answer the question, and then make several tool calls to gather more details.\n\nMinimize the number of steps, call up to 15 tools in parallel when exploring.\n\nIT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST!\n\nWhen user corrects you, acknowledge the correction, write "I will make a note to remember this" and use note_to_self call.\n\n\nExample 1\n\nUser: "What is the weather like today in Paris and London?"\nAssistant: "Must be sunny in Paris and foggy in London."\nUser: "don\'t hallucinate, use the tools"\nAssistant: "Sorry for the confusion, you are right, weather is real-time, and my best shot is to use the weather tool. I will make a note to remember this. My original instruction was to fetch weather for Paris and London. I will use 2 calls in parallel."\n[Call note_to_self "Weather" "For weather, use tools to get real-time information"]\n[Call weather "London"]\n[Call weather "Paris"]\n\n\nExample 2\n\nUser: "What is MyClass"\nAssistant: "Let me find it first."\n[Call ls "."]\nTool: subdir1, subdir2, subdir3\nAssistant: "I see 3 subdirs, will make 3 calls in parallel to check what\'s inside."\n[Call ls "subdir1"]\n[Call ls "subdir2"]\n[Call ls "subdir3"]\nTool: ...\nTool: ...\nTool: ...\nAssistant: "I give up, I can\'t find a file relevant for MyClass 😕"\nUser: "Look, it\'s my_class.cpp"\nAssistant: "Sorry for the confusion, there is in fact a file named `my_class.cpp` in `subdir2` that must be relevant for MyClass. I will make a note to remember this. My original instruction was to describe MyClass."\n[Call note_to_self "searching, browsing filesystem, MyClass" "Try to lowercase the name user asked about, convert to snake case, and guess the right file this way."]\n[Call cat "subdir2/my_class.cpp"]\nTool: ...\nAssistant: "MyClass does this and this"\n\n\n',
  ],
  ["user", "Explain what Frog is"],
  [
    "assistant",
    "Let me find the relevant information about Frog first. I will check the contents of files or directories that might contain information about Frog. I will start by listing the files in the current directory to see if there are any relevant files or directories.",
  ],
  [
    "assistant",
    "Let me find the relevant information about Frog first. I will check the contents of files or directories that might contain information about Frog. I will start by listing the files in the current directory to see if there are any relevant files or directories.",
    [
      {
        id: "call_WOyQ1sykVGppzWjjUu1drk6L",
        function: {
          arguments: '{"dirpath":"."}',
          name: "ls",
        },
        type: "function",
        index: 0,
      },
    ],
  ],
  [
    "tool",
    {
      tool_call_id: "call_WOyQ1sykVGppzWjjUu1drk6L",
      content:
        "Listing directory .\n      2260    file Cargo.toml\n      1530    file LICENSE\n       224     dir target\n      1198    file mycaps_te3.json\n       416     dir tests\n    152298    file Cargo.lock\n       757    file mycaps_openai.json\n        61    file build.rs\n      1264    file mycaps_gte.json\n      1598    file _video\n      3548    file README.md\n       768     dir examples\n       219    file _backtrace\n      1665    file _video2\n       141    file a.sh\n       139    file _help\n       992     dir src\n",
      finish_reason: "call_worked",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_IYK970zyp9vZ36m7emzmNDC9",
      content:
        'File README.md:50-99\n```    "temperature": 0.1,\n    "max_new_tokens": 20\n  }\n}\'\n```\n\nOutput is `[{"code_completion": "\\n    return \\"Hello World!\\"\\n"}]`.\n\n[LSP example](examples/lsp_completion.py)\n\n\n## Telemetry\n\nThe flags `--basic-telemetry` and `--snippet-telemetry` control what telemetry is sent. To be clear: without\nthese flags, no telemetry is sent. Those flags are typically controlled from IDE plugin settings.\n\nBasic telemetry means counters and error messages without information about you or your code. It is "compressed"\ninto `.cache/refact/telemetry/compressed` folder, then from time to time it\'s sent and moved\nto `.cache/refact/telemetry/sent` folder.\n\n"Compressed" means similar records are joined together, increasing the counter. "Sent" means the rust binary\ncommunicates with a HTTP endpoint specified in caps (see Caps section below) and sends .json file exactly how\nyou see it in `.cache/refact/telemetry`. The files are human-readable.\n\nWhen using Refact self-hosted server, telemetry goes to the self-hosted server, not to the cloud.\n\n\n## Caps File\n\nThe `--address-url` parameter controls the behavior of this program by a lot. The address is first used\nto construct `$URL/coding_assistant_caps.json` address to fetch the caps file. Furthermore, there are\ncompiled-in caps you can use by magic addresses "Refact" and "HF".\n\nThe caps file describes which models are running, default models for completion and chat,\nwhere to send the telemetry, how to download a\ntokenizer, where is the endpoint to access actual language models. To read more, check out\ncompiled-in caps in [caps.rs](src/caps.rs).\n\n\n## Tests\n\nThe one to run often is [test_edge_cases.py](tests/test_edge_cases.py).\n\nYou can also run [measure_humaneval_fim.py](tests/measure_humaneval_fim.py) for your favorite model.\n\n\n## Credits\n\nThe initial version of this project was written by looking at llm-ls by [@McPatate](https://github.com/McPatate). He\'s a Rust fan who inspired this project!\n```',
      finish_reason: "call_worked",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_8jTn7oj8tfctEnqgKQRBJH0w",
      content:
        'File Cargo.toml:39-88\n```futures-util = "0.3"\nasync-stream = "0.3.5"\nchrono = "0.4.31"\nregex = "1.9.5"\nasync-trait = "0.1.73"\nsimilar = "2.3.0"\naxum = "0.6.20"\nuuid = { version = "1", features = ["v4"] }\nlazy_static = "1.4.0"\n\nregex-automata = { version = "0.1.10", features = ["transducer"] }\nsorted-vec = "0.8.3"\ntree-sitter = "0.20"\ntree-sitter-cpp = "0.20"\n#tree-sitter-c-sharp = "0.20"\ntree-sitter-java = "0.20"\ntree-sitter-javascript = "0.20"\n#tree-sitter-kotlin = "0.3.1"\ntree-sitter-python = "0.20"\ntree-sitter-rust = "0.20"\ntree-sitter-typescript = "0.20"\n\narrow = "47.0.0"\narrow-array = "47.0.0"\narrow-schema= "47.0.0"\nasync_once= "0.2.6"\nasync-process = "2.0.1"\nitertools = "0.11.0"\nlance = "=0.9.0"\nlance-linalg = "=0.9.0"\nlance-index = "=0.9.0"\nlog = "0.4.20"\nmd5 = "0.7"\nmockito = "0.28.0"\nnotify = { version = "6.1.1", features = ["serde"] }\nparking_lot = { version = "0.12.1", features = ["serde"] }\nrusqlite = { version = "0.30.0", features = ["bundled"] }\ntempfile = "3.8.1"\ntime = "0.3.30"\ntokio-rusqlite = "0.5.0"\nvectordb = "=0.4.0"\nwalkdir = "2.3"\nwhich = "5.0.0"\nstrsim = "0.8.0"\ntypetag = "0.2"\ndyn_partial_eq = "=0.1.2"\nrayon = "1.8.0"\nbacktrace = "0.3.71"\nrand = "0.8.5"\n```',
      finish_reason: "call_worked",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_Ql7xrkn5BqtjVSHHAnNksFis",
      content:
        'File Cargo.lock:6265-6314\n```]\n\n[[package]]\nname = "zstd"\nversion = "0.11.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "20cc960326ece64f010d2d2107537f26dc589a6573a316bd5b1dba685fa5fde4"\ndependencies = [\n "zstd-safe 5.0.2+zstd.1.5.2",\n]\n\n[[package]]\nname = "zstd"\nversion = "0.12.4"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1a27595e173641171fc74a1232b7b1c7a7cb6e18222c11e9dfb9888fa424c53c"\ndependencies = [\n "zstd-safe 6.0.6",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "5.0.2+zstd.1.5.2"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "1d2a5585e04f9eea4b2a3d1eca508c4dee9592a89ef6f450c11719da0726f4db"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-safe"\nversion = "6.0.6"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "ee98ffd0b48ee95e6c5168188e44a54550b1564d9d530ee21d5f0eaed1069581"\ndependencies = [\n "libc",\n "zstd-sys",\n]\n\n[[package]]\nname = "zstd-sys"\nversion = "2.0.9+zstd.1.5.5"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "9e16efa8a874a0481a574084d34cc26fdb3b99627480f785888deb6386506656"\ndependencies = [\n "cc",\n "pkg-config",\n]\n```',
      finish_reason: "call_worked",
    },
  ],
  [
    "assistant",
    "Based on the information found in the files:\n\n- Frog seems to be a project related to coding assistance and telemetry.\n- Telemetry is sent based on flags like `--basic-telemetry` and `--snippet-telemetry`.\n- The `Caps File` contains information about models, default models for completion and chat, telemetry destinations, tokenizers, and language models.\n- Tests related to the project include `test_edge_cases.py` and `measure_humaneval_fim.py`.\n- The project credits [@McPatate](https://github.com/McPatate) for the initial version, who is a Rust fan.\n\nFor more specific details about Frog, further exploration into the project's source code or documentation may be needed.",
  ],
  ["user", "continue searching for the Frog in tests"],
  [
    "tool",
    {
      tool_call_id: "call_KKgK2Ki3bpvCk2jCtDhQxfpw",
      content:
        "Listing directory tests\n      2438    file test_hf_endpoint.py\n      3021    file lsp_connect.py\n       678    file lsp_completion.py\n         0    file __init__.py\n        96     dir __pycache__/\n      1340    file test_at_completion.py\n      3926    file post_doc_info.py\n       224     dir emergency_frog_situation/\n      4107    file test_edge_cases.py\n      2081    file code_completion_with_rag.py\n      2866    file measure_humaneval_fim.py\n",
      finish_reason: "call_worked",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_dT0OdIL7JLYJ7Fxk72MvmuRA",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_C5uTWek5PUKmaTfe7u0TypZL",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_9vYcfrFCiUhy8g6bNn0WJn5p",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  ],
  [
    "tool",
    {
      tool_call_id: "call_6Cg6UfAvNTgEt96EQrHz16W6",
      content:
        "ERROR: [Errno 21] Is a directory: './tests/emergency_frog_situation'",
      finish_reason: "call_failed",
    },
  ],
  [
    "assistant",
    "I couldn't retrieve specific information about Frog in the tests directory. If there are any other locations or specific files you suspect might contain details about Frog, please let me know.",
  ],
  ["user", "use ls, don't be stupid"],
  [
    "tool",
    {
      tool_call_id: "call_UoHvkwbPq6LMAKRM0iblVkSB",
      content:
        "Listing directory tests/emergency_frog_situation\n      1516    file jump_to_conclusions.py\n       695    file set_as_avatar.py\n        96     dir __pycache__/\n       777    file frog.py\n       249    file work_day.py\n",
      finish_reason: "call_worked",
    },
  ],
  [
    "assistant",
    "In the `tests/emergency_frog_situation` directory, the file `frog.py` seems to be relevant to Frog. Let's check the content of `frog.py` to find out more about Frog.",
  ],
  [
    "assistant",
    "In the `tests/emergency_frog_situation` directory, the file `frog.py` seems to be relevant to Frog. Let's check the content of `frog.py` to find out more about Frog.",
    [
      {
        id: "call_spx7e7LMfw97BmmzojQQf0rO",
        function: {
          arguments: '{"filepath":"tests/emergency_frog_situation/frog.py"}',
          name: "cat",
        },
        type: "function",
        index: 0,
      },
    ],
  ],
  [
    "tool",
    {
      tool_call_id: "call_spx7e7LMfw97BmmzojQQf0rO",
      content:
        "File tests/emergency_frog_situation/frog.py:1-29\n```import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n```",
      finish_reason: "call_worked",
    },
  ],
  [
    "assistant",
    "The file `frog.py` in the `tests/emergency_frog_situation` directory contains a Python class `Frog` with methods related to simulating the movement of a frog in a pond. The class includes methods for initialization, bouncing off banks, and jumping within the pond. The `Frog` class has attributes like position (`x`, `y`) and velocity (`vx`, `vy`), along with methods to handle movement and boundary conditions.\n\nThis information suggests that Frog in this context is related to a simulation or program involving a frog's behavior in a pond.",
  ],
];

export const FROG_CHAT: ChatThread = {
  id: "77b6a451-5598-44c0-bd5b-cfc19e3f4e60",
  messages: [
    [
      "context_memory",
      [
        {
          memo_id: "note20240604_call_56uppUKhQv5bGmcEBlH7QX5w.txt",
          memo_text: "Number of correction points 1\n",
        },
        {
          memo_id: "note20240604_call_WCmczUa2WUQINsoSvqq0kUPk.txt",
          memo_text: "Number of correction points 1\n",
        },
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
    ["user", "\nWhat is the difference between a frog and a toad?\n"],
    [
      "assistant",
      null,
      [
        {
          function: {
            arguments: '{"path": "frog.txt"}',
            name: "file",
          },
          id: "call_NSSpdvLovaH50zZUug463YRI",
          index: 0,
          type: "function",
        },
        {
          function: {
            arguments: '{"path": "toad.txt"}',
            name: "file",
          },
          id: "call_cmTkaNJ0roopnMcNfG4raxny",
          index: 1,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_NSSpdvLovaH50zZUug463YRI",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    ],
    [
      "tool",
      {
        tool_call_id: "call_cmTkaNJ0roopnMcNfG4raxny",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    ],
    [
      "context_file",
      [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    ],
    [
      "assistant",
      null,
      [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/toad.py"}',
            name: "file",
          },
          id: "call_8ER9PVREdkt37h84LZyc97c9",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_8ER9PVREdkt37h84LZyc97c9",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    ],
    [
      "context_file",
      [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    ],
    [
      "assistant",
      'The content of the files "frog.py" and "toad.py" seems to be the same. It appears that there was an issue with retrieving the correct content for the "toad.py" file. Let me attempt to fetch the content of the "toad.py" file again.',
      [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/toad.py"}',
            name: "file",
          },
          id: "call_1bHhD3bVIzvOueSDq1otYX4i",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_1bHhD3bVIzvOueSDq1otYX4i",
        content:
          "attached file: /Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
      },
    ],
    [
      "context_file",
      [
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
      ],
    ],
    [
      "assistant",
      'It seems that there was an issue with retrieving the content of the "toad.py" file. Since the content of the "frog.py" file has been successfully retrieved, I will analyze the content and provide the differences between a frog and a toad based on the code.\nBased on the content of the "frog.py" file, it defines a class named "Frog" with methods for initializing the frog\'s position and velocity, bouncing off the banks of a pond, and jumping within the pond. The use of the numpy library for mathematical operations is also evident.\n\nHowever, since the content of the "toad.py" file was not retrieved successfully, I am unable to provide the specific differences between a frog and a toad based on the code. It seems that there might be an issue with the file retrieval process. If there are specific characteristics or differences you would like to know about frogs and toads, please let me know, and I can attempt to gather the information from other sources.',
      null,
    ],
  ],
  title: "",
  model: "",
};

export const CHAT_WITH_DIFFS: ChatMessages = [
  [
    "assistant",
    "Persistence is essential in software development to ensure that data is stored and maintained even after the application is closed or the system is shut down.",
    null,
  ],
  [
    "context_file",
    [
      {
        file_name:
          "hibernate-orm/hibernate-core/src/test/java/org/hibernate/orm/test/id/usertype/UserTypeComparableIdTest.java",
        line1: 1,
        line2: 228,
        file_content:
          "/*\\n * Hibernate, Relational Persistence for Idiomatic Java\\n *\\n * License: GNU Lesser General Public License (LGPL), version 2.1 or later.\\n * See the lgpl.txt",
      },
    ],
  ],
  [
    "diff",
    [
      {
        file_name: "file1.py",
        file_action: "edit",
        line1: 5,
        line2: 6,
        lines_remove: "def f(x: int):\n    return x*2\n",
        lines_add: "def f(x: float):\n    return x*3\n",
      },
      {
        file_name: "file1.py",
        file_action: "edit",
        line1: 8,
        line2: 9,
        lines_remove: "def g(x: int):\n    return x*4\n",
        lines_add: "def g(x: float):\n    return x*5\n",
      },
      {
        file_name: "file2.py",
        file_action: "new",
        lines_add: "def main():\n    file1.f(6)\n",
        line1: 1,
        line2: 2,
        lines_remove: "",
      },
    ],
    "test_tool_call_id",
  ],
  ["user", "A longer diff"],
  ["assistant", "here you go"],
  [
    "diff",
    [
      {
        file_name: "long.py",
        file_action: "new",
        lines_add: "def main():    file1.f(6);    ".repeat(6),
        lines_remove: "",
        line1: 1,
        line2: 2,
      },
    ],
    "test_tool_call_id",
  ],
];

export const CHAT_WITH_DIFF_ACTIONS: ChatState["chat"] = {
  id: "eeda523e-9b74-4df6-8d60-a14ccdd907f0",
  messages: [
    ["user", "In the project add an edible property to the frog class\n"],
    [
      "assistant",
      "",
      [
        {
          function: {
            arguments: '{"query":"class Frog"}',
            name: "search_workspace",
          },
          id: "call_n5qeQaFZNAoaP3qJzRiGO6Js",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_n5qeQaFZNAoaP3qJzRiGO6Js",
        content: "performed vecdb search, results below",
      },
    ],
    [
      "context_file",
      [
        {
          file_content:
            '# Picking up context, goal in this file:\n# - goto parent class, two times\n# - dump parent class\n\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\n\n# This this a comment for the Toad class, above the class\nclass Toad(frog.Frog):\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "Bob"\n\n\nclass EuropeanCommonToad(frog.Frog):\n    """\n    This is a comment for EuropeanCommonToad class, inside the class\n    """\n\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "EU Toad"\n\n\nif __name__ == "__main__":\n    toad = EuropeanCommonToad(100, 100, 200, -200)\n    toad.jump(W, H)\n    print(toad.name, toad.x, toad.y)\n\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          line1: 1,
          line2: 32,
          usefulness: 0,
        },
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 27,
          usefulness: 0,
        },
        {
          file_content:
            '# Picking up context, goal in this file:\n# - pick up type of p\n# - prioritize type definition over all the noise\n\nimport pygame\nimport numpy as np\nimport frog\nfrom typing import Tuple\n\n\nW = 640\nH = 480\n\n\ndef draw_hello_frog(\n    screen: pygame.Surface,\n    message: str,\n    color: Tuple[int, int, int] = (0, 255, 255),\n    font_name: str = "Arial",\n) -> None:\n    font = pygame.font.SysFont(font_name, 32)\n    text = font.render(message, True, color)\n    text_rect = text.get_rect()\n    text_rect.center = (W / 2, H / 2)\n    screen.blit(text, text_rect)\n\n\ncreatures = [\n    frog.Frog(\n        np.random.uniform(0, W),\n        np.random.uniform(0, H),\n        np.random.uniform(-W/10, H/10),\n        np.random.uniform(-W/10, H/10),\n    ) for i in range(10)]\n\n\ndef main_loop():\n    screen = pygame.display.set_mode((W,H))   # without space because it\'s a test it needs to pick up right line below\n    quit_flag = False\n    while not quit_flag:\n        for event in pygame.event.get():\n            if event.type == pygame.QUIT:\n                quit_flag = True\n        screen.fill((0, 0, 0))\n        for p in creatures:\n            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)\n        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))\n        pygame.display.flip()\n        pygame.time.Clock().tick(60)\n        p: frog.Frog\n        for p in creatures:\n            p.jump(W, H)\n\n\nif __name__ == \'__main__\':\n    pygame.init()\n    pygame.display.set_caption("Pond")\n    main_loop()\n    pygame.quit()\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          line1: 1,
          line2: 58,
          usefulness: 0,
        },
        {
          file_content:
            "# Picking up context, goal in this file:\n# - without any other information, find method usage in another file by text similarity\n\nimport numpy as np\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\ndef bring_your_own_frog_to_work_day(f):\n    f.jump(W, H)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          line1: 1,
          line2: 11,
          usefulness: 0,
        },
      ],
    ],
    [
      "assistant",
      "",
      [
        {
          function: {
            arguments:
              '{"path":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py","todo":"Add an \'edible\' property to the Frog class and initialize it in the constructor."}',
            name: "patch",
          },
          id: "call_dIXVNlzugvrPJvTF5G7n1YgK",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "diff",
      [
        {
          file_action: "edit",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 1,
          lines_add:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n        self.edible = True",
          lines_remove:
            "class Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy",
        },
      ],
      "test_tool_call_id",
    ],
    [
      "assistant",
      "The `Frog` class has been updated to include an `edible` property.",
      null,
    ],
  ],
  title: "In the project add an edible property to the frog class\n",
  model: "gpt-4o",
  applied_diffs: {
    "diff-5": {
      fetching: false,
      error: "Bad Request",
      diff_id: "diff-5",
      state: [0],
      can_apply: [false],
      applied_chunks: [false],
    },
  },
  createdAt: "2024-07-05T09:10:29.523Z",
  lastUpdated: "2024-07-05T09:10:37.322Z",
};

export const LARGE_DIFF: ChatState["chat"] = {
  id: "7ebafb62-a701-43a3-9bdd-fd1425b54536",
  messages: [
    ["user", "rename the frog class to bird."],
    [
      "assistant",
      "",
      [
        {
          function: {
            arguments: "{}",
            name: "tree",
          },
          id: "call_b0ZalvpaQCZLGIHS0t4O3tH3",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_b0ZalvpaQCZLGIHS0t4O3tH3",
        content:
          "  \n    Users\n      marc\n        Projects\n          refact-lsp\n            tests\n              emergency_frog_situation\n                frog.py\n                holiday.py\n                jump_to_conclusions.py\n                set_as_avatar.py\n                work_day.py\n",
      },
    ],
    [
      "assistant",
      "",
      [
        {
          function: {
            arguments: '{"query":"class Frog"}',
            name: "search_workspace",
          },
          id: "call_YozL4pz5zNwdEaNWhdVQdcIF",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "tool",
      {
        tool_call_id: "call_YozL4pz5zNwdEaNWhdVQdcIF",
        content: "performed vecdb search, results below",
      },
    ],
    [
      "context_file",
      [
        {
          file_content:
            '# Picking up context, goal in this file:\n# - goto parent class, two times\n# - dump parent class\n\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\n\n# This this a comment for the Toad class, above the class\nclass Toad(frog.Frog):\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "Bob"\n\n\nclass EuropeanCommonToad(frog.Frog):\n    """\n    This is a comment for EuropeanCommonToad class, inside the class\n    """\n\n    def __init__(self, x, y, vx, vy):\n        super().__init__(x, y, vx, vy)\n        self.name = "EU Toad"\n\n\nif __name__ == "__main__":\n    toad = EuropeanCommonToad(100, 100, 200, -200)\n    toad.jump(W, H)\n    print(toad.name, toad.x, toad.y)\n\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          line1: 1,
          line2: 32,
          usefulness: 0,
        },
        {
          file_content:
            "import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          line1: 1,
          line2: 26,
          usefulness: 0,
        },
        {
          file_content:
            '# Picking up context, goal in this file:\n# - pick up type of p\n# - prioritize type definition over all the noise\n\nimport pygame\nimport numpy as np\nimport frog\nfrom typing import Tuple\n\n\nW = 640\nH = 480\n\n\ndef draw_hello_frog(\n    screen: pygame.Surface,\n    message: str,\n    color: Tuple[int, int, int] = (0, 255, 255),\n    font_name: str = "Arial",\n) -> None:\n    font = pygame.font.SysFont(font_name, 32)\n    text = font.render(message, True, color)\n    text_rect = text.get_rect()\n    text_rect.center = (W / 2, H / 2)\n    screen.blit(text, text_rect)\n\n\ncreatures = [\n    frog.Frog(\n        np.random.uniform(0, W),\n        np.random.uniform(0, H),\n        np.random.uniform(-W/10, H/10),\n        np.random.uniform(-W/10, H/10),\n    ) for i in range(10)]\n\n\ndef main_loop():\n    screen = pygame.display.set_mode((W,H))   # without space because it\'s a test it needs to pick up right line below\n    quit_flag = False\n    while not quit_flag:\n        for event in pygame.event.get():\n            if event.type == pygame.QUIT:\n                quit_flag = True\n        screen.fill((0, 0, 0))\n        for p in creatures:\n            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)\n        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))\n        pygame.display.flip()\n        pygame.time.Clock().tick(60)\n        p: frog.Frog\n        for p in creatures:\n            p.jump(W, H)\n\n\nif __name__ == \'__main__\':\n    pygame.init()\n    pygame.display.set_caption("Pond")\n    main_loop()\n    pygame.quit()\n',
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          line1: 1,
          line2: 58,
          usefulness: 0,
        },
        {
          file_content:
            "# Picking up context, goal in this file:\n# - without any other information, find method usage in another file by text similarity\n\nimport numpy as np\nimport frog\n\nX,Y = 50, 50\nW = 100\nH = 100\n\ndef bring_your_own_frog_to_work_day(f):\n    f.jump(W, H)\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          line1: 1,
          line2: 11,
          usefulness: 0,
        },
        {
          file_content:
            "import frog\n\n\nif __name__ == __main__:\n    frog1 = frog.Frog()\n    frog2 = frog.Frog()\n\n    # First jump\n    frog1.jump()\n    frog2.jump()\n\n    # Second jump\n    frog1.jump()\n    frog2.jump()\n\n    # Third jump\n    frog1.jump()\n    frog2.jump()\n\n    # Forth jump\n    frog1.jump()\n    frog2.jump()\n",
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          line1: 1,
          line2: 21,
          usefulness: 0,
        },
      ],
    ],
    [
      "assistant",
      "",
      [
        {
          function: {
            arguments:
              '{"paths":"/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py,/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py","symbols":"Frog","todo":"Rename the class Frog to Bird and update all its references accordingly."}',
            name: "patch",
          },
          id: "call_GViOZuuRlNT5tGK7RXQZxkhu",
          index: 0,
          type: "function",
        },
      ],
    ],
    [
      "diff",
      [
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "class Frog:\n",
          lines_add: "class Bird:\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 13,
          line2: 14,
          lines_remove: "class Toad(frog.Frog):\n",
          lines_add: "class Toad(bird.Bird):\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/set_as_avatar.py",
          file_action: "edit",
          line1: 19,
          line2: 20,
          lines_remove: "class EuropeanCommonToad(frog.Frog):\n",
          lines_add: "class EuropeanCommonToad(bird.Bird):\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 7,
          line2: 8,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 29,
          line2: 30,
          lines_remove: "    frog.Frog(\n",
          lines_add: "    bird.Bird(\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/jump_to_conclusions.py",
          file_action: "edit",
          line1: 50,
          line2: 51,
          lines_remove: "        p: frog.Frog\n",
          lines_add: "        p: bird.Bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/work_day.py",
          file_action: "edit",
          line1: 5,
          line2: 6,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          file_action: "edit",
          line1: 1,
          line2: 2,
          lines_remove: "import frog\n",
          lines_add: "import frog as bird\n",
        },
        {
          file_name:
            "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
          file_action: "edit",
          line1: 5,
          line2: 7,
          lines_remove: "    frog1 = frog.Frog()\n    frog2 = frog.Frog()\n",
          lines_add: "    frog1 = bird.Bird()\n    frog2 = bird.Bird()\n",
        },
      ],
      "call_GViOZuuRlNT5tGK7RXQZxkhu",
    ],
    [
      "assistant",
      "The class `Frog` has been successfully renamed to `Bird` and all its references have been updated accordingly in the following files:\n\n- `frog.py`\n- `set_as_avatar.py`\n- `jump_to_conclusions.py`\n- `work_day.py`\n- `holiday.py`\n\nIs there anything else you need help with?",
      null,
    ],
  ],
  title: "rename the frog class to bird.\n",
  model: "gpt-4o",
  applied_diffs: {
    "diff-7": {
      fetching: false,
      error: null,
      diff_id: "diff-7",
      state: [],
      applied_chunks: [
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
      ],
      can_apply: [true, true, true, true, true, true, true, true, true, true],
    },
  },
  createdAt: "2024-07-23T15:08:51.480Z",
  lastUpdated: "2024-07-23T15:36:26.738Z",
};
