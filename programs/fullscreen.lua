dofile("programs/lin_alg.lua")
dofile("programs/rainbow_cube.lua")

function reload()
    if init == nil then
        anim = 0.0
        fullscreen = track_shader("shaders/fullscreen_tri.vert", "shaders/neat_pattern.frag", "tri")
        init = true
    end
end

function frame()
    anim = anim + 1.0
    return {
        anim=anim,
        {
            n_indices=3,
            shader=fullscreen,
        },
    }
end
