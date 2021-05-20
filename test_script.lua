function reload()
    if anim == nil then anim = 0.0 end
    if mesh == nil then 
        data = rainbow_cube()
        mesh = add_mesh(data[1], data[2])
    end
end

function frame()
    anim = anim + 0.01
    return {
        {
            {
                math.cos(anim), 0., -math.sin(anim), 0.,
                0., 1., 0., 0.,
                math.sin(anim), 0., math.cos(anim), 0.,
                0., math.sin(anim * 3.), 0., 1.,
            },
            mesh,
        },
        {
            {
                1., 0., 0., 0.,
                0., 1., 0., 0.,
                0., 0., 1., 0.,
                3., 0., 0., 1.,
            },
            mesh,
        }
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
