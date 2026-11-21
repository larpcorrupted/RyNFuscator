local count = 0
local names = {"alpha", "beta", "gamma"}
for i = 1, #names do
  count = count + 1
end
print(count, names[2])

local function add(a, b)
  return a + b
end
print(add(19, 23))

local msg = "hello world"
print(msg, #msg)

for n = 1, 3 do
  print("tick", n)
end

local t = { x = 1, y = 2 }
do
  t.x = t.x + 10
  t.y = t.y * 2
end
print(t.x, t.y)

print(100, 42, 7)
