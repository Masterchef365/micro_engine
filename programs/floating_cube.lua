dofile("programs/lin_alg.lua")
dofile("programs/rainbow_cube.lua")

function reload()
    midi_states = {}
    for i = 1, 16 do
        midi_states[i] = 0.0
    end

    if init == nil then
        anim = 0.0
        mesh = add_mesh(table.unpack(rainbow_cube()))
        shader = track_shader("shaders/unlit.vert", "shaders/unlit.frag", "tri")
        init = true
    end
end

function midi(data)
    midi_states[data.msg[2]+1] = data.msg[3]
end

function frame()
    anim = anim + 0.01
    return {
        anim=anim,
        {
            trans=cannon(translate(0, midi_states[1] / 80., 0)),
            -- trans=cannon(gemm(
            --     translate(0, midi_states[1] / 80., 0),
            --     rot_y(anim)
            -- )),
            mesh=mesh,
            shader=shader,
        },
    }
end
