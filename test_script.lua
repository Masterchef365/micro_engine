dofile("lin_alg.lua")

objs = {}

function reload()
    if init == nil then
        anim = 0.0

        data = rainbow_cube()
        mesh = add_mesh(data[1], data[2])

        init = true
    end
end

function frame()
    anim = anim + 0.01
    return {
        {
            cannon(gemm(
                translate(0, math.sin(anim), 0),
                rot_y(anim)
            )),
            mesh,
        },
        {
            cannon(translate(3, 0, 0)),
            mesh,
        },
        table.unpack(objs)
    }
end

function rainbow_cube()
    return {
        {
            -1.0, -1.0, -1.0, 0.0, 1.0, 1.0,
            1.0, -1.0, -1.0, 1.0, 0.0, 1.0,
            1.0, 1.0, -1.0, 1.0, 1.0, 0.0,
            -1.0, 1.0, -1.0, 0.0, 1.0, 1.0,
            -1.0, -1.0, 1.0, 1.0, 0.0, 1.0,
            1.0, -1.0, 1.0, 1.0, 1.0, 0.0,
            1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
            -1.0, 1.0, 1.0, 1.0, 0.0, 1.0
        },
        {
            3, 1, 0, 2, 1, 3, 2, 5, 1, 6, 5, 2, 6, 4, 5, 7, 4, 6, 7, 0, 4, 3, 0, 7, 7, 2, 3, 6, 2, 7,
            0, 5, 4, 1, 5, 0,
        }
    }
end
