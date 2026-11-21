-- game exists but ScriptContext is missing -> trip code 2
local chunk = io.open(arg[1], "r"):read("*a")
local env = setmetatable({}, { __index = _G })
env.print = function(...) io.write(table.concat({...}, "\t"), "\n") end
env.game = setmetatable({}, {
  __index = function(_, k)
    if k == "GetService" then
      return function() error("no service") end
    end
    if k == "FindService" then
      return function() return nil end
    end
    return nil
  end,
})
env.task = { wait = function() error("halted") end }
env.getgenv = function() return env end

local f = assert(load(chunk, "chunk", "t", env))
local ok, err = pcall(f)
print("ok=" .. tostring(ok))
print("err=" .. tostring(err))
print("flag=" .. tostring(env.obfuscatenow))
