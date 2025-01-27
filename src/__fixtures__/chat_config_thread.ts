import type { Chat } from "../features/Chat/Thread";

export const CHAT_CONFIG_THREAD: Chat = {
  streaming: false,
  thread: {
    mode: "CONFIGURE",
    id: "941fb8f4-409c-4430-a3b2-6450fafdb9f4",
    messages: [
      {
        role: "user",
        content:
          "🔧 The postgres tool should be visible now. To test the tool, list the tables available, briefly desctibe the tables and express\nsatisfaction and relief if it works, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.\nThe current config file is .\n",
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_IkNfXpwhNVR6D1Sr2CDA5Cfi",
            function: {
              arguments:
                '{"query":"\\nSELECT table_name\\nFROM information_schema.tables\\nWHERE table_schema=\'public\';"}',
              name: "postgres",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_IkNfXpwhNVR6D1Sr2CDA5Cfi",
          content:
            "🧩 for configuration go to SETTINGS:postgres, psql failed:\nNo such file or directory (os error 2)",
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_kw6TJChemYjXEGL9mLL3T0mN",
            function: {
              arguments: '{"path":"/Users/marc/Projects/refact-lsp"}',
              name: "tree",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_kw6TJChemYjXEGL9mLL3T0mN",
          content:
            "/\n  Users/\n    marc/\n      Projects/\n        refact-lsp/\n          .dockerignore\n          .gitattributes\n          .gitignore\n          CODE_OF_CONDUCT.md\n          CONTRIBUTING.md\n          Cargo.lock\n          Cargo.toml\n          Cross.toml\n          INTEGRATIONS.md\n          LICENSE\n          README.md\n          build.rs\n          tests/\n            __init__.py\n            lsp_connect.py\n            test01_completion_edge_cases.py\n            test02_completion_with_rag.py\n            test03_at_commands_completion.py\n            test04_completion_lsp.py\n            test05_is_openai_compatible.py\n            test06_tool_not_tool.py\n            test07_memories.py\n            test08_post_processing.py\n            test09_ast_pick_up_changes.py\n            test10_locate.py\n            test11_patch.py\n            test11_patch_partial_edit.py\n            test12_tools_authorize_calls.py\n            test13_vision.py\n            test_diff_handlers.py\n            test13_data/\n              200.jpg\n              530.jpg\n            test11_data/\n              already_applied_rewrite_symbol_01.py\n              already_applied_rewrite_symbol_02.py\n              toad_orig.py\n              toad_partial_edit_01.py\n              toad_partial_edit_02.py\n              toad_rewrite_symbol_01.py\n              toad_rewrite_symbol_02.py\n              toad_rewrite_symbol_03.py\n              toad_rewrite_symbol_04_orig.rs\n              toad_rewrite_symbol_04_patched.rs\n            emergency_frog_situation/\n              frog.py\n              holiday.py\n              jump_to_conclusions.py\n              set_as_avatar.py\n              work_day.py\n          src/\n            background_tasks.rs\n            cached_tokenizers.rs\n            call_validation.rs\n            caps.rs\n            completion_cache.rs\n            custom_error.rs\n            diffs.rs\n            fetch_embedding.rs\n            file_filter.rs\n            files_correction.rs\n            files_in_jsonl.rs\n            files_in_workspace.rs\n            forward_to_hf_endpoint.rs\n            forward_to_openai_endpoint.rs\n            fuzzy_search.rs\n            git.rs\n            global_context.rs\n            http.rs\n            knowledge.rs\n            known_models.rs\n            lsp.rs\n            main.rs\n            nicer_logs.rs\n            privacy.rs\n            privacy_compiled_in.rs\n            restream.rs\n            scratchpad_abstract.rs\n            subchat.rs\n            version.rs\n            yaml_configs/\n              create_configs.rs\n              customization_compiled_in.rs\n              customization_loader.rs\n              mod.rs\n            vecdb/\n              mod.rs\n              vdb_cache.rs\n              vdb_file_splitter.rs\n              vdb_highlev.rs\n              vdb_lance.rs\n              vdb_remote.rs\n              vdb_structs.rs\n              vdb_thread.rs\n            tools/\n              mod.rs\n              tool_ast_definition.rs\n              tool_ast_reference.rs\n              tool_cat.rs\n              tool_cmdline.rs\n              tool_deep_thinking.rs\n              tool_knowledge.rs\n              tool_locate_search.rs\n              tool_patch.rs\n              tool_relevant_files.rs\n              tool_search.rs\n              tool_tree.rs\n              tool_web.rs\n              tools_description.rs\n              tools_execute.rs\n              tool_patch_aux/\n                ast_lint.rs\n                diff_apply.rs\n                diff_structs.rs\n                fs_utils.rs\n                mod.rs\n                no_model_edit.rs\n                postprocessing_utils.rs\n                tickets_parsing.rs\n                model_based_edit/\n                  blocks_of_code_parser.rs\n                  mod.rs\n                  model_execution.rs\n                  partial_edit.rs\n                  whole_file_parser.rs\n            telemetry/\n              basic_comp_counters.rs\n              basic_network.rs\n              basic_robot_human.rs\n              basic_transmit.rs\n              mod.rs\n              snippets_collection.rs\n              snippets_transmit.rs\n              telemetry_structs.rs\n              utils.rs\n            scratchpads/\n              chat_generic.rs\n              chat_llama2.rs\n              chat_passthrough.rs\n              chat_utils_deltadelta.rs\n              chat_utils_limit_history.rs\n              chat_utils_prompts.rs\n              code_completion_fim.rs\n              code_completion_replace.rs\n              comments_parser.rs\n              mod.rs\n              multimodality.rs\n              passthrough_convert_messages.rs\n              scratchpad_utils.rs\n            postprocessing/\n              mod.rs\n              pp_command_output.rs\n              pp_context_files.rs\n              pp_plain_text.rs\n              pp_utils.rs\n            integrations/\n              config_chat.rs\n              integr_abstract.rs\n              integr_chrome.rs\n              integr_github.rs\n              integr_gitlab.rs\n              integr_pdb.rs\n              integr_postgres.rs\n              mod.rs\n              process_io_utils.rs\n              running_integrations.rs\n              sessions.rs\n              setting_up_integrations.rs\n              yaml_schema.rs\n              docker/\n                docker_container_manager.rs\n                docker_ssh_tunnel_utils.rs\n                integr_docker.rs\n                mod.rs\n            http/\n              routers.rs\n              utils.rs\n              routers/\n                info.rs\n                v1.rs\n                v1/\n                  ast.rs\n                  at_commands.rs\n                  at_tools.rs\n                  caps.rs\n                  chat.rs\n                  code_completion.rs\n                  code_lens.rs\n                  customization.rs\n                  dashboard.rs\n                  docker.rs\n                  git.rs\n                  graceful_shutdown.rs\n                  gui_help_handlers.rs\n                  handlers_memdb.rs\n                  links.rs\n                  lsp_like_handlers.rs\n                  patch.rs\n                  snippet_accepted.rs\n                  status.rs\n                  subchat.rs\n                  sync_files.rs\n                  system_prompt.rs\n                  telemetry_network.rs\n                  v1_integrations.rs\n                  vecdb.rs\n            dashboard/\n              dashboard.rs\n              mod.rs\n              structs.rs\n              utils.rs\n            at_commands/\n              at_ast_definition.rs\n              at_ast_reference.rs\n              at_commands.rs\n              at_file.rs\n              at_search.rs\n              at_tree.rs\n              at_web.rs\n              execute_at.rs\n              mod.rs\n            ast/\n              ast_db.rs\n              ast_indexer_thread.rs\n              ast_parse_anything.rs\n              ast_structs.rs\n              chunk_utils.rs\n              dummy_tokenizer.json\n              file_splitter.rs\n              linters.rs\n              mod.rs\n              parse_common.rs\n              parse_python.rs\n              treesitter/\n                ast_instance_structs.rs\n                file_ast_markup.rs\n                language_id.rs\n                mod.rs\n                parsers.rs\n                skeletonizer.rs\n                structs.rs\n                parsers/\n                  cpp.rs\n                  java.rs\n                  js.rs\n                  python.rs\n                  rust.rs\n                  tests.rs\n                  ts.rs\n                  utils.rs\n                  tests/\n                    cpp.rs\n                    java.rs\n                    js.rs\n                    python.rs\n                    rust.rs\n                    ts.rs\n                    cases/\n                      ts/\n                        main.ts\n                        main.ts.json\n                        person.ts\n                        person.ts.decl_json\n                        person.ts.skeleton\n                      rust/\n                        main.rs\n                        main.rs.json\n                        point.rs\n                        point.rs.decl_json\n                        point.rs.skeleton\n                      python/\n                        calculator.py\n                        calculator.py.decl_json\n                        calculator.py.skeleton\n                        main.py\n                        main.py.json\n                      js/\n                        car.js\n                        car.js.decl_json\n                        car.js.skeleton\n                        main.js\n                        main.js.json\n                      java/\n                        main.java\n                        main.java.json\n                        person.java\n                        person.java.decl_json\n                        person.java.skeleton\n                      cpp/\n                        circle.cpp\n                        circle.cpp.decl_json\n                        circle.cpp.skeleton\n                        main.cpp\n                        main.cpp.json\n              alt_testsuite/\n                cpp_goat_library.correct\n                cpp_goat_library.h\n                cpp_goat_main.correct\n                cpp_goat_main.cpp\n                jump_to_conclusions_annotated.py\n                py_goat_library.correct\n                py_goat_library.py\n                py_goat_library_annotated.py\n                py_goat_main.py\n                py_goat_main_annotated.py\n                py_torture1_attr.py\n                py_torture1_attr_annotated.py\n                py_torture2_resolving.py\n                py_torture2_resolving_annotated.py\n          python_binding_and_cmdline/\n            setup.py\n            refact/\n              __init__.py\n              chat_client.py\n              cli_app_switcher.py\n              cli_export.py\n              cli_inspect.py\n              cli_main.py\n              cli_markdown.py\n              cli_printing.py\n              cli_settings.py\n              cli_statusbar.py\n              cli_streaming.py\n              lsp_runner.py\n              traj_compressor.py\n          examples/\n            ast_definition.sh\n            ast_references.sh\n            chat_with_at_command.py\n            http_caps.sh\n            http_chat.sh\n            http_chat_passthrough.sh\n            http_completion.sh\n            http_rag_status.sh\n            http_subchat.sh\n            http_vecdb_search.sh\n            lsp_runner.py\n            note3.py\n            rag_skeletonize_video.py\n          docker/\n            lsp-debug.Dockerfile\n          chrome/\n            mac_arm-130.0.6723.69/\n              chrome-mac-arm64/\n                Google Chrome for Testing.app/\n                  Contents/\n                    Resources/\n                      com.google.chrome.for.testing.manifest/\n                        Contents/\n                          Resources/\n                            com.google.chrome.for.testing.manifest\n                            en.lproj/\n                              Localizable.strings\n                    Frameworks/\n                      Google Chrome for Testing Framework.framework/\n                        Versions/\n                          130.0.6723.69/\n                            Libraries/\n                              WidevineCdm/\n                                _platform_specific/\n                                  mac_arm64/\n                                    libwidevinecdm.dylib\n          bring_your_own_key/\n            hf.yaml\n            mixed.yaml\n            openai.yaml\n            openrouter.yaml",
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_4we2wH5H50A2m6CIJqKbzYH8",
            function: {
              arguments: '{"paths": "Cargo.toml", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
          {
            id: "call_vmIGl31ytfpLWPkc138HJnxz",
            function: {
              arguments: '{"paths": "README.md", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 1,
          },
          {
            id: "call_QD1oyHwPOvvFdYUfV3ijiKzB",
            function: {
              arguments:
                '{"paths": "tests/emergency_frog_situation/frog.py", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 2,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_QD1oyHwPOvvFdYUfV3ijiKzB",
          content:
            "Paths found:\n/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/frog.py\n",
        },
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_vmIGl31ytfpLWPkc138HJnxz",
          content:
            'Path problems:\n\nThe path "README.md" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_4we2wH5H50A2m6CIJqKbzYH8",
          content:
            'Path problems:\n\nThe path "Cargo.toml" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "context_file",
        content: [
          {
            file_name: "frog.py",
            file_content:
              'import numpy as np\n\nDT = 0.01\n\nclass Frog:\n    def __init__(self, x, y, vx, vy):\n        self.x = x\n        self.y = y\n        self.vx = vx\n        self.vy = vy\n\n    def bounce_off_banks(self, pond_width, pond_height):\n        if self.x < 0:\n            self.vx = np.abs(self.vx)\n        elif self.x > pond_width:\n            self.vx = -np.abs(self.vx)\n        if self.y < 0:\n            self.vy = np.abs(self.vy)\n        elif self.y > pond_height:\n            self.vy = -np.abs(self.vy)\n\n    def jump(self, pond_width, pond_height):\n        self.x += self.vx * DT\n        self.y += self.vy * DT\n        self.bounce_off_banks(pond_width, pond_height)\n        self.x = np.clip(self.x, 0, pond_width)\n        self.y = np.clip(self.y, 0, pond_height)\n\n    def croak(self, n_times):\n        for n in range(n_times):\n            print("croak")\n\n\nclass AlternativeFrog:\n    def alternative_jump(self):\n        pass\n\n\ndef standalone_jumping_function():\n    print("I\'m a frog! Jump! Jump! Really!")\n',
            line1: 1,
            line2: 39,
          },
        ],
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_sNnvyOLlmulfHIikJzKhmz55",
            function: {
              arguments: '{"paths": "src/Cargo.toml", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
          {
            id: "call_n9FACVDl6a4tllgyWaLCl8s9",
            function: {
              arguments: '{"paths": "src/main.rs", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 1,
          },
          {
            id: "call_1DUkSfLq6Io8xowqVjJEuybM",
            function: {
              arguments: '{"paths": "src/lib.rs", "symbols": ""}',
              name: "cat",
            },
            type: "function",
            index: 2,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_1DUkSfLq6Io8xowqVjJEuybM",
          content:
            'Path problems:\n\nThe path "src/lib.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_n9FACVDl6a4tllgyWaLCl8s9",
          content:
            'Path problems:\n\nThe path "src/main.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_sNnvyOLlmulfHIikJzKhmz55",
          content:
            'Path problems:\n\nThe path "src/Cargo.toml" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_h8biZ8JNUjeWGbL9CCatbPcf",
            function: {
              arguments: '{"paths":"src/background_tasks.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_h8biZ8JNUjeWGbL9CCatbPcf",
          content:
            'Path problems:\n\nThe path "src/background_tasks.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_coI5PDRF2O2Mjcfb8bdZ4Heo",
            function: {
              arguments:
                '{"paths":"src/tools/integr_postgres.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_coI5PDRF2O2Mjcfb8bdZ4Heo",
          content:
            'Path problems:\n\nThe path "src/tools/integr_postgres.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_RXDRwr3yCJEplHud4xRoxWlC",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_postgres.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_RXDRwr3yCJEplHud4xRoxWlC",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_postgres.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_OvYAforNKAFz0d7mv96AnTVR",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_abstract.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_OvYAforNKAFz0d7mv96AnTVR",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_abstract.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_3wuMILBYl0z7pBMUoJOwohQf",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_github.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_3wuMILBYl0z7pBMUoJOwohQf",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_github.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_AzBRj8RiL1wc8eejuBVdaS6t",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_gitlab.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_AzBRj8RiL1wc8eejuBVdaS6t",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_gitlab.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_bJ2MrMSJHk4IF6Gp5DNLhJZP",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_chrome.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_bJ2MrMSJHk4IF6Gp5DNLhJZP",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_chrome.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_kZFTzONZdyo11FQcSudIo3vK",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_pdb.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_kZFTzONZdyo11FQcSudIo3vK",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_pdb.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_MDynldaxbGEuCKSuQg0Vgk5z",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_docker.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_MDynldaxbGEuCKSuQg0Vgk5z",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_docker.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_9fCGZwstx7G1MgHs6JD5JWTn",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_abstract.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_9fCGZwstx7G1MgHs6JD5JWTn",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_abstract.rs" does not exist. There are no similar names either.\n',
        },
      },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          {
            id: "call_etmMcI1UwBSaWwZHzxsuL8xu",
            function: {
              arguments:
                '{"paths":"src/integrations/integr_postgres.rs","symbols":""}',
              name: "cat",
            },
            type: "function",
            index: 0,
          },
        ],
      },
      {
        role: "tool",
        content: {
          tool_call_id: "call_etmMcI1UwBSaWwZHzxsuL8xu",
          content:
            'Path problems:\n\nThe path "src/integrations/integr_postgres.rs" does not exist. There are no similar names either.\n',
        },
      },
    ],
    title:
      "🔧 The postgres tool should be visible now. To test the tool, list the tables available, briefly desctibe the tables and express\nsatisfaction and relief if it works, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.\nThe current config file is .\n",
    model: "",
    tool_use: "agent",
    integration: {
      name: "postgres",
      path: "",
      project: "",
    },
    read: true,
    createdAt: "2024-12-02T14:42:18.902Z",
    updatedAt: "2024-12-02T14:42:18.902Z",
  },
  error: null,
  prevent_send: true,
  waiting_for_response: false,
  max_new_tokens: 4096,
  cache: {},
  system_prompt: {},
  tool_use: "agent",
  send_immediately: false,
};
