local M = {}

M.base_30 = {
  white         = "#DEDEDE",
  darker_black  = "#26272d",
  black         = "#282A36", -- nvim bg
  black2        = "#2d303e",
  one_bg        = "#373844", -- real bg of onedark
  one_bg2       = "#44475a",
  one_bg3       = "#565761",
  grey          = "#5e5f69",
  grey_fg       = "#666771",
  grey_fg2      = "#6e6f79",
  light_grey    = "#73747e",
  red           = "#fa8294",
  baby_pink     = "#ff86d3",
  pink          = "#FF79C6",
  line          = "#3c3d49", -- for lines like vertsplit
  green         = "#50fa7b",
  vibrant_green = "#5dff88",
  nord_blue     = "#8b9bcd",
  blue          = "#a1b1e3",
  yellow        = "#F1FA8C",
  sun           = "#FFFFA5",
  purple        = "#BD93F9",
  dark_purple   = "#BD93F9",
  teal          = "#92a2d4",
  orange        = "#FFB86C",
  cyan          = "#8BE9FD",
  statusline_bg = "#2d2f3b",
  lightbg       = "#41434f",
  pmenu_bg      = "#b389ef",
  folder_bg     = "#BD93F9",
}

M.base_16 = {
  base00 = "#2c2d34",
  base01 = "#3a3c4e",
  base02 = "#4d4f68",
  base03 = "#626483",
  base04 = "#62d6e8",
  base05 = "#d4d4e9", -- Fg color for terminal and sidebar
  base06 = "#f1f2f8",
  base07 = "#f7f7fb",
  base08 = "#72D1Fd",
  base09 = "#E36FFA",
  base0E = "#9eefc0",
  base0B = "#F3B9FB",
  base0C = "#8BE9FD",
  base0D = "#62B2F8",
  base0A = "#ff86d3",
  base0F = "#DEDEDE",
}

M.type = "dark"

M.polish_hl = {
  ["@function.builtin"] = { fg = M.base_30.cyan },
  ["@number"] = { fg = M.base_30.purple },
  ["Structure"] = { fg = M.base_16.base0A },
  ["Special"] = { fg = "#C461F5"},
  ["PreProc"] = { fg = M.base_30.sun },
  ["@constant.macro"] = { fg = "#9678f0" },
  ["@variable"] = { fg = M.base_16.base0C  },
  ['@lsp.mod.mutable'] = { fg="#CF7BFF", bold = true, force=true },
  ['@lsp.typemod.variable.mutable'] = { fg="#CF7BFF", bold = true, force=true }
}

return M
