local on_attach = require("plugins.configs.lspconfig").on_attach
local capabilities = require("plugins.configs.lspconfig").capabilities
 -- local utils = require("core.util")

local lspconfig = require "lspconfig"
local util = require "lspconfig/util"

-- if you just want default config for the servers then put them in a table
local servers = { "html", "cssls", "tsserver" }

for _, lsp in ipairs(servers) do
  lspconfig[lsp].setup {
    on_attach = on_attach,
    capabilities = capabilities,
  }
end

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

lspconfig["glsl_analyzer"].setup {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = { "glsl", "vert", "tesc", "tese", "frag", "geom", "comp" },
  single_file_support = true,
  cmd = {"/opt/glsl_analyzer/bin/glsl_analyzer" }
}

lspconfig["clangd"].setup {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = { "c", "cpp", "objective-c", "objective-cpp", "metal", "cuda", "metal", "msl" },
  root_dir =  util.root_pattern(
      '.clangd',
      '.clang-tidy',
      '.clang-format',
      'compile_commands.json',
      'compile_flags.txt',
      '.git'
    )
}

-- 
-- Setup rust_analyzer
-- lspconfig["rust_analyzer"].setup {
--   on_attach = on_attach,
--   capabilities = capabilities,
--   filetypes = {"rust"},
--   root_dir = util.root_pattern("Cargo.toml"),
--   setings = {
--     ['rust-analyzer'] = {
--       cargo = {
--         allFeatures = true,
--       },
--       diagnostics = {
--         enable = false;
--       }
--     }
--   }
-- }
