dofile("programs/lin_alg.lua")
dofile("programs/icosahedron.lua")

function reload()
    if init == nil then
        anim = 0.0
        cube = icosahedron(3.0, "tris")
        mesh = add_mesh(cube[1], cube[2])
        shader = track_shader("shaders/unlit.vert", "shaders/unlit.frag", "triangles")
        init = true
    end
end

function frame()
    anim = anim + 0.03
    local objs = {anim=anim}
    objs[1] = {
        trans=cannon(gemm(
            translate(0, math.cos(anim) / 5., 0),
            rot_y(anim)
        )),
        mesh=mesh,
        shader=shader,
    }
    return objs
end
