local overrides = require("custom.configs.overrides")

---@type NvPluginSpec[]
local plugins = {
  {
    "neovim/nvim-lspconfig",
    config = function()
      require "plugins.configs.lspconfig"
      require "custom.configs.lspconfig"
    end, -- Override to setup mason-lspconfig
  },
  {
    "williamboman/mason.nvim",
    opts = overrides.mason
  },

  {
    "nvim-treesitter/nvim-treesitter",
    opts = overrides.treesitter,
  },
  {
    "nvim-tree/nvim-tree.lua",
    opts = overrides.nvimtree,
  },
  {
    "max397574/better-escape.nvim",
    event = "InsertEnter",
    config = function()
      require("better_escape").setup()
    end,
  },
  {
    "stevearc/conform.nvim",
    --  for users those who want auto-save conform + lazyloading!
    -- event = "BufWritePre"
    config = function()
      require "custom.configs.conform"
    end,
  },
  {
    "rust-lang/rust.vim",
    ft = "rust",
    init = function()
      vim.g.rustfmt_autosave = 1
    end
  },
  {
    "mrcjkb/rustaceanvim",
    version = '^4.26',
    ft = {"rust"},
  },
  {
    "mfussenegger/nvim-dap",
  },
  {
    "rcarriga/nvim-dap-ui",
    keys = {
      { "<leader>du", function() require("dapui").toggle({ }) end, desc = "Dap UI toggle" },
      { "<leader>de", function() require("dapui").eval() end, desc = "Eval", mode = {"n", "v"} },
    },
    opts = {},
    config = function(_, opts)
      local dap = require("dap")
      local dapui = require("dapui")
      dapui.setup(opts)
      dap.listeners.after.event_initialized["dapui_config"] = function()
        dapui.open({})
      end
      dap.listeners.before.event_terminated["dapui_config"] = function()
        dapui.close({})
      end
      dap.listeners.before.event_exited["dapui_config"] = function()
        dapui.close({})
      end
    end,
  },
  {
    "saecki/crates.nvim",
    ft = {"rust", "toml" },
    config = function(_, opts)
      local crates = require 'crates';
      crates.setup(opts)
      crates.show()
    end
  },
  {
    "hrsh7th/nvim-cmp",
    opts = function()
      local M = require 'plugins.configs.cmp'
      table.insert(M.sources, {name = "crates"})
      return M
    end,
  },
  {
    'MeanderingProgrammer/render-markdown.nvim',
    dependencies = { 'nvim-treesitter/nvim-treesitter', 'echasnovski/mini.nvim' }, -- if you use the mini.nvim suite
    opts = {},
  },
  {
    "olimorris/codecompanion.nvim",
    dependencies = {
      "nvim-lua/plenary.nvim",
      "nvim-treesitter/nvim-treesitter",
      { "MeanderingProgrammer/render-markdown.nvim", ft = { "codecompanion" } },
    },
    cmd = { "CodeCompanion", "CodeCompanionChat", "CodeCompanionActuions" },
    keys = {
      { "<C-r>", "<cmd>CodeCompanionActions<cr>" },
      { "<C-r>", "<cmd>CodeCompanionActions<cr>", mode = "v" },
      { "<Leader>cc", "<cmd>CodeCompanionChat Toggle<cr>" },
      { "<Leader>cc", "<cmd>CodeCompanionChat Toggle<cr>", mode = "v" },
      { "ga", "<cmd>CodeCompanionChat Add<cr>", mode = "v" },
    },
    opts = {
      adapters = {
        copilot = function()
          return require("codecompanion.adapters").extend("copilot", {
            schema = {
              model = {
                default = "claude-3.7-sonnet",
              },
            },
          })
        end,
      },
      prompt_library = {
        ["Rust"] = {
          strategy = "chat",
          description = "A Rust assistant",
          opts = {
            index = 11,
            is_slash_cmd = true,
            auto_submit = false,
            short_name = "rust",
            ignore_system_prompt = true,
          },
          prompts = {
            {
              role = "system",
              content = [[
You are an expert Rust programmer and helpful AI assistant. Your primary goal is to assist users with various aspects of Rust development, ensuring code quality, correctness, safety, and adherence to idiomatic Rust practices.

**Core Responsibilities:**

1.  **Code Generation & Refactoring:** Write clean, efficient, and idiomatic Rust code. Refactor existing code for clarity, performance, or safety improvements. Adhere to the Rust edition specified by the user (default to the latest stable edition, currently 2021, if unspecified).
2.  **Explanation & Teaching:** Clearly explain Rust concepts (e.g., ownership, borrowing, lifetimes, traits, generics, async/await, macros, error handling) with examples. Tailor explanations to the user's apparent skill level.
3.  **Debugging & Error Analysis:** Help diagnose and fix compilation errors (including complex lifetime or borrow checker issues), runtime errors (panics), and logic bugs. Analyze error messages and suggest concrete solutions.
4.  **Best Practices & Idioms:** Guide users towards idiomatic Rust patterns and away from common pitfalls. Explain *why* certain patterns are preferred.
5.  **Cargo & Ecosystem:** Assist with `Cargo.toml` configuration, dependency management, feature flags, workspaces, and leveraging the Rust ecosystem (crates.io).
6.  **Testing:** Help write effective unit, integration, and documentation tests using Rust's built-in testing framework or other popular testing crates.
7.  **Performance:** Offer advice on performance optimization, benchmarking techniques, and understanding potential performance implications of different code constructs.
8.  **Safety & `unsafe`:** Emphasize Rust's safety guarantees. If `unsafe` code is necessary, explain the associated risks, invariants that must be upheld, and best practices for its usage.
9.  **API Design:** Provide guidance on designing clear, robust, and ergonomic Rust APIs.

**Guidelines for Interaction:**

*   **Prioritize Safety & Correctness:** Ensure generated code is memory-safe and logically correct.
*   **Idiomatic Code:** Write code that follows standard Rust conventions and style guidelines (e.g., `rustfmt`).
*   **Error Handling:** Prefer `Result<T, E>` for recoverable errors. Use `panic!` only for unrecoverable errors or programming bugs. Explain the chosen error handling strategy.
*   **Clarity:** Write clear, well-commented code, especially for complex logic or `unsafe` blocks. Explain your reasoning and the generated code.
*   **Context-Awareness:** Pay attention to the user's existing code, project context, and constraints (e.g., `no_std` environments).
*   **Ask Clarifying Questions:** If the user's request is ambiguous, ask for more details before proceeding.
*   **Iterative Refinement:** Be prepared to refine code or explanations based on user feedback.
*   **Crate Suggestions:** When appropriate, suggest relevant crates from the ecosystem, but explain the trade-offs.

**Your Goal:** Be a reliable, knowledgeable, and helpful partner in the user's Rust development journey. Help them write better, safer, and more efficient Rust code.
              ]]
            },
          },
        },
        ["Kernel"] = {
          strategy = "chat",
          description = "A C++ CUDA and HIP kernel developer assistant",
          opts = {
            index = 11,
            is_slash_cmd = true,
            auto_submit = false,
            short_name = "kernel",
            ignore_system_prompt = true,
          },
          prompts = {
            {
              role = "system",
              content = [[
You are an expert C++ programmer specializing in high-performance GPU kernel development (CUDA, HIP, potentially OpenCL/SYCL). Your primary goal is to assist users in writing, optimizing, and debugging C++ kernels, with a strong emphasis on leveraging target-specific inline assembly (NVIDIA PTX or AMD GCN/RDNA ISA) where it provides tangible performance benefits over standard C++ or vendor API intrinsics.

**Core Responsibilities:**

1.  **Kernel Code Generation & Refactoring:** Write efficient, correct, and well-structured C++ GPU kernel code. Refactor existing kernels for performance, clarity, or correctness. Optimize for target GPU architectures (specify if known, e.g., NVIDIA Ampere, AMD RDNA2).
2.  **Inline Assembly Integration:**
    *   Identify performance bottlenecks in C++ kernel code where specific hardware instructions, accessed via inline assembly (PTX for NVIDIA, GCN/RDNA assembly for AMD), could offer significant improvements (e.g., specialized texture/memory operations, fused operations, specific data movement instructions, bit manipulation).
    *   Generate correct and safe inline assembly snippets within the C++ kernel context.
    *   Clearly document the purpose of the inline assembly, the specific instructions used, input/output operands, and any clobbered registers.
    *   Explain *why* the assembly version is expected to be faster than the C++ equivalent (e.g., avoids compiler limitations, uses hardware features directly).
    *   Provide C++ fallback implementations where feasible, perhaps using `#ifdef` blocks, for portability or comparison.
3.  **Explanation & Teaching:** Clearly explain GPU architecture concepts (warps/wavefronts, SIMT/SIMD, memory hierarchy: global, shared/LDS, texture, constant, registers), parallel programming paradigms, synchronization primitives (barriers, atomics, shuffles), kernel launch configurations, and the intricacies of CUDA/HIP APIs.
4.  **Debugging & Error Analysis:** Help diagnose and fix kernel-specific issues: race conditions, bank conflicts, memory divergence, incorrect synchronization, precision errors, out-of-bounds memory access on the GPU, and issues arising from incorrect inline assembly usage.
5.  **Performance Optimization:** Guide users on GPU performance optimization techniques: memory coalescing, maximizing occupancy, minimizing thread divergence, efficient use of shared memory/LDS, instruction-level parallelism, kernel fusion, and effective use of vendor profiling tools (like Nsight Compute/Systems, rocprof).
6.  **Best Practices:** Promote best practices for robust and maintainable GPU kernel development, including careful handling of memory, synchronization, and error checking within the constraints of the C++/GPU environment. Advise on safe integration of inline assembly.
7.  **API & Build Systems:** Assist with CUDA/HIP Runtime/Driver API usage, kernel launch syntax, and integrating GPU code compilation (e.g., `nvcc`, `hipcc`) into build systems like CMake.

**Guidelines for Interaction:**

*   **Prioritize Correctness & Performance:** Ensure generated code, especially with inline assembly, is functionally correct *and* achieves the intended performance goal. Acknowledge potential trade-offs.
*   **Target Architecture Awareness:** When generating or optimizing assembly, ask for or state assumptions about the target GPU architecture (e.g., "This PTX is optimized for Ampere", "This GCN assembly targets RDNA2"). Performance characteristics can vary significantly.
*   **Justify Inline Assembly:** Do not use inline assembly gratuitously. Only recommend it when there's a clear, demonstrable performance reason or necessity to access hardware features not exposed otherwise. Explain the rationale clearly.
*   **Safety with Assembly:** Emphasize the potential dangers of inline assembly (compiler inability to reason about it, potential for subtle bugs, register clobbering). Ensure constraints and assumptions are documented.
*   **Clarity:** Write clear C++ code and heavily comment any inline assembly sections. Explain the kernel logic and optimization strategies.
*   **Context-Awareness:** Pay attention to existing code structure, data types, and constraints mentioned by the user.
*   **Portability Caveats:** Explicitly mention that direct inline assembly severely impacts portability between GPU vendors (and sometimes even architectures from the same vendor).

**Your Goal:** Be a highly specialized and reliable assistant for advanced C++ GPU kernel development. Help users push the performance boundaries of their target hardware by correctly and judiciously applying inline PTX or AMD assembly, while maintaining code correctness and providing clear explanations.
              ]]
            },
          },
        },
      },
      strategies = {
        -- Change the default chat adapter
        chat = {
          adapter = "copilot",
          slash_commands = {
            ["buffer"] = {
              opts = {
                provider = "telescope",
                keymaps = {
                  modes = {
                    i = "<C-b>",
                  },
                },
              },
            },
            ["help"] = {
              opts = {
                provider = "telescope",
                max_lines = 1000,
              },
            },
            ["file"] = {
              opts = {
                provider = "telescope",
              },
            },
            ["symbols"] = {
              opts = {
                provider = "telescope",
              },
            },
          },
        },
      },
      display = {
        action_palette = {
          provider = "telescope",
        },
        chat = {
          -- show_references = true,
          -- show_header_separator = false,
          -- show_settings = false,
        },
        diff = {
          provider = "mini_diff",
        },
      },
      opts = {
        -- Set debug logging
        log_level = "DEBUG",
      },
    },
  },
  {
    "echasnovski/mini.diff", -- Inline and better diff over the default
    config = function()
      local diff = require("mini.diff")
      diff.setup({
        -- Disabled by default
        source = diff.gen_source.none(),
      })
    end,
  },

  -- copilot
  {
      "zbirenbaum/copilot.lua",
      cmd = "Copilot",
      event = "InsertEnter",
      --opts = overrides.copilot, -- have your own local overrided configurations in: custom/configs/overrides.lua
      config = function()
          require("copilot").setup({
              panel = {
                  enabled = false,
                  auto_refresh = false,
                  keymap = {
                      jump_prev = "[[",
                      jump_next = "]]",
                      accept = "<CR>",
                      refresh = "gr",
                      open = "<M-CR>"
                  },
                  layout = {
                      position = "right", -- | top | left | right
                      ratio = 0.4
                  },
              },
              suggestion = {
                  enabled = true,
                  auto_trigger = true, -- if autocomplete menu doesn't show while you type, setting this to true is recommended
                  debounce = 75,
                  keymap = {
                      accept = "<C-a>",
                      accept_word = "C-y>",
                      accept_line = "<C-u>",
                      next = "<M-]>",
                      prev = "<M-[>",
                      dismiss = "<C-x>",
                  },
              },
              filetypes = {
                  yaml = false,
                  markdown = false,
                  help = false,
                  gitcommit = false,
                  gitrebase = false,
                  hgcommit = false,
                  svn = false,
                  cvs = false,
                  javascript = true, -- allow specific filetype
                  typescript = true, -- allow specific filetype
                  rust = true,
                  c = true,
                  ["."] = false,
                  --["*"] = false, -- disable for all other filetypes and ignore default `filetypes`
                  sh = function ()
                      if string.match(vim.fs.basename(vim.api.nvim_buf_get_name(0)), '^%.env.*') then
                          -- disable for .env files
                          return false
                      end
                      return true
                  end,
              },
              copilot_node_command = 'node', -- Node.js version must be > 16.x
              server_opts_overrides = {
                  trace = "verbose",
                  settings = {
                      advanced = {
                          listCount = 6, -- #completions for panel
                          inlineSuggestCount = 5, -- #completions for getCompletions
                      }
                  },
              },
          })
      end,
  },
}

return plugins
