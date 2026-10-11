local first = 1
local second = 2 -- trailing

-- attached
if first < second then
    work()
elseif first == second then
    same()
else
    other()
end

local values = {
    first,
    second,
}

save(
    values
)

return values
