---@type ChadrcConfig
local M = {}

-- Path to overriding theme and highlights files
local highlights = require "custom.highlights"

vim.api.nvim_set_hl(0, '@lsp.mod.mutable', { fg="#C96BFF", bold = true, force = true })
vim.api.nvim_set_hl(0, '@lsp.typemod.variable.mutable', { link = "@lsp.mod.mutable" })
vim.api.nvim_set_hl(0, '@lsp.type.property', { fg="#55A7FF" })
vim.api.nvim_set_hl(0, '@lsp.type.selfTypeKeyword', { fg="#9eefc0" })
vim.api.nvim_set_hl(0, '@lsp.type.builtInType', { fg="#9eefc0" })

M.ui = {
  theme = "spyral",
  theme_toggle = { "doomchad", "one_light" },

  hl_override = highlights.override,
  hl_add = highlights.add,
}

M.plugins = "custom.plugins"

-- check core.mappings for table structure
M.mappings = require "custom.mappings"

return M
