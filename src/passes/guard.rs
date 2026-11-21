use crate::rng::Rng;

const TRIP_TYPE: i64 = 1;
const TRIP_MISSING: i64 = 2;
const TRIP_DEAD: i64 = 3;

pub fn build(rng: &mut Rng, roblox: bool) -> String {
    if !roblox {
        return String::new();
    }

    let mode = fresh(rng);
    let dev = fresh(rng);
    let crash = fresh(rng);
    let expect = fresh(rng);
    let guard = fresh(rng);
    let wait = fresh(rng);

    let t1 = TRIP_TYPE;
    let t2 = TRIP_MISSING;
    let t3 = TRIP_DEAD;

    format!(
        r#"local {mode} = rawget(_G, "RYN_FAIL_MODE") or "hang"
local {dev} = rawget(_G, "RYN_DEV") or false
local {guard} = {{}}
local function {crash}(code)
  if {dev} then
    return code
  end
  if type(getgenv) == "function" then
    local ok, env = pcall(getgenv)
    if ok and type(env) == "table" then
      env.obfuscatenow = "PROTECT AT [HTTPS://github.com/larpcorrupted/RyNFuscator] !!!!"
    end
  end
  if {mode} == "error" then
    error(code, 0)
  end
  local w = (type(task) == "table" and type(task.wait) == "function") and task.wait or nil
  if not w then
    error(code, 0)
  end
  while true do
    w(60000)
  end
end
local function {expect}(cond, reason)
  if not cond then {crash}(reason) end
  return cond
end
local function {wait}()
  if type(game) ~= "userdata" and type(game) ~= "table" then
    return
  end
  local hits = 0
  local ok, res = pcall(function()
    local s = game:GetService("ScriptContext")
    return s.GetCoverageStats(s)
  end)
  if ok then
    hits = hits + 1
    {expect}(typeof(res) == "table", {t1})
  elseif not game:FindService("ScriptContext") then
    {crash}({t2})
  end
  local ok2 = pcall(function()
    local a = game:GetService("AnalyticsService")
    return a
  end)
  if ok2 then
    hits = hits + 1
  elseif not game:FindService("AnalyticsService") then
    {crash}({t2})
  end
  {expect}(hits >= 1, {t3})
  return {guard}
end
{wait}()
"#,
        mode = mode,
        dev = dev,
        crash = crash,
        expect = expect,
        guard = guard,
        wait = wait,
        t1 = t1,
        t2 = t2,
        t3 = t3
    )
}

fn fresh(rng: &mut Rng) -> String {
    const A: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";
    let len = 3 + rng.below(3);
    let mut s = String::from("_");
    for _ in 0..len {
        s.push(A[rng.below(A.len())] as char);
    }
    s
}

pub fn already_guarded(src: &str) -> bool {
    src.contains("RYN_FAIL_MODE") || src.contains("obfuscatenow")
}

#[allow(dead_code)]
pub fn validate(src: &str) -> Result<(), crate::lexer::LexError> {
    crate::lexer::tokenize(src).map(|_| ())
}
