local overrides = require("custom.configs.overrides")

---@type NvPluginSpec[]
local plugins = {

  -- Override plugin definition options

  {
    "neovim/nvim-lspconfig",
    config = function()
      require "plugins.configs.lspconfig"
      require "custom.configs.lspconfig"
    end, -- Override to setup mason-lspconfig
  },

  -- override plugin configs
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

  -- Install a plugin
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
    "hiphish/rainbow-delimiters.nvim",
    config = function()
      require 'rainbow-delimiters.setup'.setup {
        query = {
            [''] = 'rainbow-delimiters',
            rust = 'rainbow-blocks',
        },
        highlight = {
          'RainbowDelimiterRed',
          'RainbowDelimiterYellow',
          'RainbowDelimiterBlue',
          'RainbowDelimiterOrange',
          'RainbowDelimiterGreen',
          'RainbowDelimiterViolet',
          'RainbowDelimiterCyan',
        },
      }
    end,
  },
  {
    "rust-lang/rust.vim",
    ft = "rust",
    init = function()
      vim.g.rustfmt_autosave = 1
    end
  },
  --{
   -- "simrat39/rust-tools.nvim",
   -- ft = "rust",
   -- dependencies = "neovim/nvim-lspconfig",
   -- opts = function()
   --   return require "custom.configs.rust-tools"
   -- end,
   -- config = function(_, opts)
   --   require 'rust-tools'.setup(opts)
   -- end
  --},
  {
    "mrcjkb/rustaceanvim",
    version = '^4',
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
--  {
--    "rouge8/neotest-rust",
--    config = function()
--      require 'neotest'.setup{
--        adapters = {
--          require('rustaceanvim.neotest') {
--            args = { "--no-capture" }
--          }
--        }
--      }
--    end
--  }

  -- To make a plugin not be loaded
  -- {
  --   "NvChad/nvim-colorizer.lua",
  --   enabled = false
  -- },

  -- All NvChad plugins are lazy-loaded by default
  -- For a plugin to be loaded, you will need to set either `ft`, `cmd`, `keys`, `event`, or set `lazy = false`
  -- If you want a plugin to load on startup, add `lazy = false` to a plugin spec, for example
  -- {
  --   "mg979/vim-visual-multi",
  --   lazy = false,
  -- }
}

return plugins
