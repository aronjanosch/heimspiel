-- ES-DE on Hyprland, for heimspiel (https://github.com/aronjanosch/heimspiel).
--
-- Hyprland keeps one fullscreen window per workspace. A game started from ES-DE takes that slot,
-- and Hyprland does not hand it back when the game exits, so ES-DE comes back tiled. This file
-- makes ES-DE fullscreen again once every window that opened on its workspace after it has
-- closed, and floats windows that open there without fullscreen, centred, instead of tiling them
-- next to the black ES-DE. (With misc:on_focus_under_fullscreen = 1, as Omarchy sets it, a window
-- opening above fullscreen ES-DE takes over its fullscreen instead.)
--
-- Load it at the end of ~/.config/hypr/hyprland.lua:
--   dofile("/path/to/heimspiel/contrib/hyprland/es-de.lua")
--
-- Windows opened before ES-DE (the terminal it was started from) do not count, so ES-DE need not
-- be alone on its workspace. Games are never forced into fullscreen; that is their choice.

local function is_esde(w)
  return w.class == "es-de" or w.class == "org.es_de.frontend" or w.class == "ES-DE"
end

local function esde_on(workspace_id)
  for _, w in ipairs(hl.get_windows()) do
    if w.mapped and is_esde(w) and w.workspace.id == workspace_id then
      return w
    end
  end
end

-- Windows that opened on ES-DE's workspace: games, their launchers and heimspiel's loading
-- window, by address.
local launched = {}

hl.on("window.open", function(w)
  if is_esde(w) or not esde_on(w.workspace.id) then
    return
  end
  launched[w.address] = true
  if w.fullscreen == 0 and not w.floating then
    local target = "address:" .. w.address
    hl.dispatch(hl.dsp.window.float({ action = "set", window = target }))
    hl.dispatch(hl.dsp.window.center({ window = target }))
  end
end)

-- window.close, not window.destroy: the window passed to the latter has no fields left.
hl.on("window.close", function(w)
  if not launched[w.address] then
    return
  end
  launched[w.address] = nil
  if next(launched) then
    return
  end
  for _, esde in ipairs(hl.get_windows()) do
    if esde.mapped and is_esde(esde) and esde.fullscreen ~= 2 then
      hl.dispatch(hl.dsp.window.fullscreen({
        window = "address:" .. esde.address, action = "set", mode = "fullscreen",
      }))
    end
  end
end)
