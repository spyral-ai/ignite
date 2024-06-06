---@type MappingsTable
local M = {}

M.general = {
  n = {
    [";"] = { ":", "enter command mode", opts = { nowait = true } },

    --  format with conform
    ["<leader>fm"] = {
      function()
        require("conform").format()
      end,
      "formatting",
    },

    -- toggle breakpoint
    ["<leader>tb"] = {
      "<cmd> DapToggleBreakpoint <CR>",
      "Toggle breakpoint"
    },

    -- debug continue
    ["<S-h>"] = {
      "<cmd> DapContinue <CR>",
      "Debugger continue"
    },

    -- debug step over
    ["<S-j>"] = {
      "<cmd> DapStepOver <CR>",
      "Debugger step over"
    },

    -- debug step into
    ["<S-k>"] = {
      "<cmd> DapStepInto <CR>",
      "Debugger step into"
    },

    -- debug step out
    ["<S-l>"] = {
      "<cmd> DapStepOut <CR>",
      "Debugger step out"
    },

    -- open debuggables list
    ["<leader>rdb"] = {
      "<cmd> RustLsp debuggables <CR>",
      "Open rust debuggables"
    },

    -- open debuggables list
    ["<leader>rru"] = {
      "<cmd> RustLsp runnables <CR>",
      "Open rust runnables"
    },

    -- show rust inlay hints
    ["<leader>sh"] = {
      function()
        local bufnr = vim.api.nvim_get_current_buf()
        local is_enabled = vim.lsp.inlay_hint.is_enabled{ bufnr };
        vim.lsp.inlay_hint.enable(not is_enabled, { bufnr = bufnr });
      end,
      "Toggle inlay hints"
    },

    -- open markdown preview window
    ["<leader>mp"] = {
      function()
        local peek = require 'peek'
        peek.open()
      end,
      "Open Markdown preview"
    },

    -- close markdown preview
    ["<leader>mpc"] = {
      function()
        local peek = require 'peek'
        peek.close()
      end,
      "Close Markdown preview"
    },

    -- rust hover actions
    ["<leader>ha"] = {
      "<cmd> RustLsp hover actions <CR>",
      "Rust hover actions"
    },

    -- rust code actions
    ["<leader>ca"] = {
      "<cmd> RustLsp codeAction <CR>",
      "Rust code actions"
    },

    -- rust proc macro expansion
    ["<leader>pm"] = {
      "<cmd> RustLsp expandMacro",
      "Rust expand proc macros recursively"
    },

    -- go to definition
    ["<leader>gd"] = {
      function()
        vim.lsp.buf.definition()
      end,
      "Go to definition",
      {
        buffer = vim.api.nvim_get_current_buf()

      }
    }
  },
  v = {
    [">"] = { ">gv", "indent"},
  },
}

vim.keymap.set(
  "n",
  "<leader>a",
  function()
    vim.cmd.RustLsp('codeAction')
    -- vim.lsp.buf.codeAction()
  end,
  { silent = true, buffer = vim.api.nvim_get_current_buf() }
)

-- more keybinds!
M.crates = {
  n = {
    ["<leader>rcu"] = {
      function()
        require 'crates'.upgrade_all_crates()
      end,
      "Update crates"
    }
  }
}

return M
