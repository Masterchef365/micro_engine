dofile("programs/lin_alg.lua")
dofile("programs/rainbow_cube.lua")

function reload()
    if init == nil then
        anim = 0.0
        mesh = add_mesh(table.unpack(rainbow_cube()))
        shader = track_shader("shaders/unlit.vert", "shaders/unlit.frag", "tri")
        init = true
    end
end

function frame()
    anim = anim + 0.01

    objs = {}
    golden = (1. + math.sqrt(5.)) / 2.
    n = math.floor(1000. * ((math.cos(anim / 2.) + 1.) / 2.))
    qn = n

    for i = 1, n do
        theta = 2. * math.pi * i / golden
        phi = math.acos(1 - 2*(i + 0.5) / qn)
        --phi = phi + anim / 10.
        -- theta = theta * math.cos(anim / 1000.)

        sz = 50. 
        --sz = sz * math.cos(phi + theta + anim / 10.)

        x = math.cos(theta) * math.sin(phi) * sz
        y = math.sin(theta) * math.sin(phi) * sz
        z = math.cos(phi) * sz

        if i <= qn then
            objs[i] = {
                --cannon(translate(x, y, z)),
                trans=cannon(gemm(translate(x, y, z), rot_y(theta))),
                shader=shader,
                mesh=mesh
            }
        end
    end

    return objs
end
