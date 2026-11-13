local settings = {
    enabled = true,
    retries = 3,
    animation = {
        duration = 0.2,
        easing = "Quad",
    }, -- trailing
    -- attached
    onComplete = function()
        print("done")
    end,
    label = "Ready",
    visible = true,
}
