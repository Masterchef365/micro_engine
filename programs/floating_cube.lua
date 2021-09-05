dofile("programs/lin_alg.lua")
dofile("programs/rainbow_cube.lua")

function reload()
    if init == nil then
        anim = 0.0
        cube = rainbow_cube()
        mesh = add_mesh(cube[1], cube[2])
        shader = track_shader("shaders/unlit.vert", "shaders/unlit.frag", "tri")
        init = true
    end
end

function frame()
    anim = anim + 0.01
    objs = {anim=anim}
    for i = 1, 1000 do
        objs[i] = {
            trans=cannon(gemm(
                translate(0, i, 0),
                rot_y(anim + i)
            )),
            mesh=mesh,
            shader=shader,
        }
    end
    return objs
end
