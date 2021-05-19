function init()
    mesh = rainbow_cube()
    key = add_mesh(mesh[1], mesh[2])
end

function frame()
    return {
        {
            {
                1., 0., 0., 0.,
                0., 1., 0., 0.,
                0., 0., 1., 0.,
                0., 0., 0., 1.,
            },
            key,
        },
        {
            {
                1., 0., 0., 0.,
                0., 1., 0., 0.,
                0., 0., 1., 0.,
                3., 0., 0., 1.,
            },
            key,
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