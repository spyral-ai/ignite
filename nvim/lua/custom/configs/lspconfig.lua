local on_attach = require("plugins.configs.lspconfig").on_attach
local capabilities = require("plugins.configs.lspconfig").capabilities

-- Servers with simple default config
local servers = { "html", "cssls", "tsserver" }

for _, server in ipairs(servers) do
  vim.lsp.config(server, {
    on_attach = on_attach,
    capabilities = capabilities,
  })
end

-- GLSL analyzer
vim.lsp.config("glsl_analyzer", {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = { "glsl", "vert", "tesc", "tese", "frag", "geom", "comp" },
  single_file_support = true,
  cmd = { "/opt/glsl_analyzer/bin/glsl_analyzer" },
})

-- Clangd
vim.lsp.config("clangd", {
  on_attach = on_attach,
  capabilities = capabilities,
  filetypes = {
    "c", "cpp", "objective-c", "objective-cpp", "metal", "cuda", "msl"
  },
  root_dir = vim.fs.root(0, {
    ".clangd",
    ".clang-tidy",
    ".clang-format",
    "compile_commands.json",
    "compile_flags.txt",
    ".git",
  }),
})

-- Enable these servers globally
vim.lsp.enable {
  "html",
  "cssls",
  "tsserver",
  "glsl_analyzer",
  "clangd",
}

-- Diagnostics keymaps
vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float)
vim.keymap.set('n', '[d', vim.diagnostic.goto_prev)
vim.keymap.set('n', ']d', vim.diagnostic.goto_next)

-- Rust code actions (if needed)
local bufnr = vim.api.nvim_get_current_buf()
vim.keymap.set(
  'n',
  '<leader>a',
  function()
    vim.cmd.RustLsp('codeAction')
  end,
  { silent = true, buffer = bufnr }
)
