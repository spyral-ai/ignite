local on_attach = require("plugins.configs.lspconfig").on_attach
local capabilities = require("plugins.configs.lspconfig").capabilities
 -- local utils = require("core.util")

local lspconfig = require "lspconfig"
local util = require "lspconfig/util"

-- Define servers that don’t need special setup
local servers = { "html", "cssls", "tsserver" }

-- Register standard servers using the new API
for _, lsp in ipairs(servers) do
  vim.lsp.config(lsp, {
    on_attach = on_attach,
    capabilities = capabilities,
  })
end

-- GLSL analyzer setup
vim.lsp.config("glsl_analyzer", {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = { "glsl", "vert", "tesc", "tese", "frag", "geom", "comp" },
  single_file_support = true,
  cmd = { "/opt/glsl_analyzer/bin/glsl_analyzer" },
})

-- Clangd setup
vim.lsp.config("clangd", {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = { "c", "cpp", "objective-c", "objective-cpp", "metal", "cuda", "msl" },
  root_dir = vim.fs.root(0, {
    ".clangd",
    ".clang-tidy",
    ".clang-format",
    "compile_commands.json",
    "compile_flags.txt",
    ".git",
  }),
})

-- Enable all servers (Neovim 0.11+)
vim.lsp.enable {
  "html",
  "cssls",
  "tsserver",
  "glsl_analyzer",
  "clangd",
}

-- Global mappings.
-- See `:help vim.diagnostic.*` for documentation on any of the below functions
vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float)
vim.keymap.set('n', '[d', vim.diagnostic.goto_prev)
vim.keymap.set('n', ']d', vim.diagnostic.goto_next)

local bufnr = vim.api.nvim_get_current_buf()
vim.keymap.set(
  'n', 
  '<leader>a', 
  function()
    vim.cmd.RustLsp('codeAction')
  end,
  {silent = true, buffer = bufnr}
)
