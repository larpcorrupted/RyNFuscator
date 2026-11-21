local out = {}

local function classify(n)
  if n == 0 then
    return "zero"
  elseif n < 0 then
    return "negative"
  else
    return "positive"
  end
end

for _, v in ipairs({-5, 0, 12, 99}) do
  table.insert(out, classify(v))
end
print(table.concat(out, ","))

local acc = 0
local i = 0
while i < 5 do
  i = i + 1
  acc = acc + i
end
print(acc)

local words = "the quick brown fox"
local parts = {}
for w in string.gmatch(words, "%a+") do
  parts[#parts + 1] = w
end
print(#parts, parts[4])

local nested = { a = { b = { c = "deep value" } } }
print(nested.a.b.c)
