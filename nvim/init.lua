require "core"

local custom_init_path = vim.api.nvim_get_runtime_file("lua/custom/init.lua", false)[1]

if custom_init_path then
  dofile(custom_init_path)
end

require("core.utils").load_mappings()

local lazypath = vim.fn.stdpath "data" .. "/lazy/lazy.nvim"

-- bootstrap lazy.nvim!
if not vim.loop.fs_stat(lazypath) then
  require("core.bootstrap").gen_chadrc_template()
  require("core.bootstrap").lazy(lazypath)
end

dofile(vim.g.base46_cache .. "defaults")
vim.opt.rtp:prepend(lazypath)
require "plugins"

vim.wo.relativenumber = true

vim.api.nvim_set_option_value("colorcolumn", "100", {})
vim.api.nvim_create_autocmd({ "BufNewFile", "BufRead" }, {
  pattern = "*.wgsl",
  callback = function()
    vim.bo.filetype = "wgsl"
  end,
})
vim.api.nvim_create_autocmd({ "BufNewFile", "BufRead" }, {
  pattern = "*.metal",
  callback = function()
    vim.bo.filetype = "cpp"
  end,
})

local vim = vim
local Plug = vim.fn['plug#']

vim.call('plug#begin')

-- Shorthand notation; fetches https://github.com/junegunn/vim-easy-align
Plug('tikhomirov/vim-glsl')
Plug('kbenzie/vim-spirv')

vim.call('plug#end')
