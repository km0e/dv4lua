---@class UserCfg
---@field mount string
---@field os string
---@field hid string
---@field is_system boolean?
---
---@class ExecOptions
---@field reply boolean
---@field etor string?

---@class User
---@field exec fun(this: User, cmd: string, opt:boolean|ExecOptions?)
---@field read fun(this: User, path: string): string
---@field write fun(this: User, path: string, content: string)
---@field user string
---@field os string
---@field [string] string
---
---@class UM
---@field add_cur fun(this: UM, cfg: table)
---@field add_ssh fun(this: UM, uid: string, cfg: table)
---@field [string] User

---@class Dot
---@field confirm fun(this: Dot, default: string)
---@field add_schema fun(this: Dot, name: string, path: string)
---@field add_source fun(this: Dot, name: string, path: string)
---@field sync fun(this: Dot, apps: table, uid: string)
---@field upload fun(this: Dot, apps: table, uid: string)

---@class Pm
---@field install fun(this: Pm, hid: string, apps: string, confirm: boolean)
---@field update fun(this: Pm, hid: string, confirm: boolean)
---@field upgrade fun(this: Pm, hid: string, apps: string, confirm: boolean)

---@class Dv
---@field sync fun(this: Dv, src: string, src_paths: string|table, dest: string, dest_paths: string|table, confirm: string?)
---@field dl fun(this: Dv, url: string, expire?: string)
---@field um fun(this: Dv):UM
---@field dot fun(this: Dv):Dot
---@field pm fun(this: Dv):Pm
---@field json fun(this: Dv, text: string|any):table|string
dv = dv

UTB = {}

---@param name string
---@return UserCfg
local function default_user_cfg(name)
  return {
    mount = "~/.local/share/dv",
    os = "linux",
    hid = name,
  }
end

local function init_user_table()
  local base = {
    mount = "~/.local/share/dv",
  }
  UTB["cur"] = base

  base.hid = "local"
  UTB["system"] = base

  local rt = {
    mount = base.mount,
    os = "ubuntu",
    hid = "rt",
  }
  UTB["rt"] = rt

  rt.is_system = false
  UTB["rt-r"] = rt
end

init_user_table()

function Load_user(...)
  local um = dv:um()
  for _, uid in ipairs({ ... }) do
    local cfg = UTB[uid] or default_user_cfg(uid)
    if uid == "cur" then
      um:add_cur(cfg)
    else
      um:add_ssh(uid, cfg)
    end
  end
  return um
end

---@param source string
function Load_dot(source)
  local schema = dv:dl("https://raw.githubusercontent.com/km0e/schema/main/dot.toml", "7days")
  local dot = dv:dot()
  dot:add_schema("cur", schema)
  dot:add_schema("rt", "~/.local/share/dv/schema.toml")
  dot:add_source("rt", source)
  return dot
end

function Main()
  local um = Load_user("cur", "system", "rt")
  local dot = Load_dot("~/.local/share/dv/main")
  if um.cur.os == "windows" then
    dot:confirm("uy")
    dot:upload({ "alacritty", "git", "nvim" }, "cur")
  else
    dot:confirm("yu")
    dot:upload({
      "alacritty",
      "fish",
      "fcitx5",
      "git",
      "google-chrome",
      "jujutsu",
      "konsole",
      "nvim",
      "yakuake",
      "zellij",
    }, "cur")
  end
end

local function latest_fish_tar()
  local um = Load_user("cur")
  local path = dv:dl("https://api.github.com/repos/fish-shell/fish-shell/releases/latest", "1days")
  local j = dv:json(um.cur:read(path))
  local target = nil
  for _, asset in ipairs(j["assets"]) do
    if asset["name"]:match("static%-amd64") then
      target = asset
    end
  end
  if not target then
    error("No suitable asset found for fish-shell.")
  end
  return {
    name = target["name"],
    path = dv:dl(target["browser_download_url"], "7days"),
  }
end
