local value <const> = 8 // 2
::again::
value = value & 3
if value > 0 then
    goto again
end
return value
