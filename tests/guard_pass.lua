-- simulates a Roblox-ish environment with the probes present
local chunk = io.open(arg[1], "r"):read("*a")

local env = setmetatable({}, { __index = _G })
env.print = function(...) io.write(table.concat({...}, "\t"), "\n") end
env.game = { FindService = function() return false end, GetService = function() error("none") end }
env.workspace = {}
env.Instance = {}
env.Enum = {}
env.task = { wait = function() error("halted") end }
env.getgenv = function() return env end
env.getfenv = function() return env end
env.loadstring = load
env.typeof = function(v) return type(v) end

local f = assert(load(chunk, "chunk", "t", env))
local ok, err = pcall(f)
print("ok=" .. tostring(ok))
print("err=" .. tostring(err))
print("ryn_guard=" .. tostring(env.ryn_guard))
