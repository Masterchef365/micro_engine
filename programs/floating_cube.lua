dofile("programs/lin_alg.lua")
dofile("programs/rainbow_cube.lua")

function reload()
    if init == nil then
        anim = 0.0
        mesh = add_mesh(table.unpack(rainbow_cube()))
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
    }
end
