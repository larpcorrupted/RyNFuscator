-- simulates a plain Lua environment: no game, workspace, Instance, Enum
local ran = false
local chunk = io.open(arg[1], "r"):read("*a")

local env = setmetatable({}, { __index = _G })
env.print = function(...) io.write(table.concat({...}, "\t"), "\n") end
env.task = { wait = function() error("halted") end }

local f = assert(load(chunk, "chunk", "t", env))
local ok, err = pcall(f)
print("ok=" .. tostring(ok))
print("err=" .. tostring(err))
