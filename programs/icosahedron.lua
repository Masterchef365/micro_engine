dofile("programs/lin_alg.lua")

function reload()
    if init == nil then
        anim = 0.0
    end
    cube = icosahedron()
    mesh = add_mesh(cube[1], cube[2])
    shader = track_shader("shaders/unlit.vert", "shaders/unlit.frag", "lines")
    init = true
end

function frame()
    anim = anim + 0.01
    local objs = {anim=anim}
    for i = 1, 1 do
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

function icosahedron()
    -- Color
    local color = { 1., 1., 1. }

    -- Edge length
    local l = 1.0

    local tau = math.pi*2

    -- Distance to a vertex from radial center
    local r = (1/math.sin(tau/(5*2))) * (l/2)

    -- Vertical distance between a plane and the vertex centered above it 
    local h1 = math.sqrt(math.pow(l, 2) - math.pow(r, 2))

    -- Distance between the inner vertical layers
    local h2 = math.sqrt(math.pow(l, 2) - math.pow((l/2), 2))

    local tstep = (tau/5)

    local layers = { 
        { t=0, y=h2 / 2 }, 
        { t=tstep / 2, y=-h2 / 2 }, 
    }

    local vertices = {}

    function addcolor()
        for k = 1, #(color) do
            table.insert(vertices, color[k])
        end
    end

    -- Inner layer vertices
    for j = 1,2 do
        local layer = layers[j]

        for i = 0,4 do
            local t = i * tstep + layer.t

            local pos = { 
                math.cos(t) * r,
                layer.y,
                math.sin(t) * r,
            }

            for k = 1, #pos do
                table.insert(vertices, pos[k])
            end

            addcolor()
        end
    end

    -- Top and bottom vertices 
    local height = h2/2. + h1
    local top_bot = {
        { 0.0, height, 0.0 },
        { 0.0, -height, 0.0 },
    }

    for j = 1, 2 do
        local pos = top_bot[j]
        for k = 1, #pos do
            table.insert(vertices, pos[k])
        end
        addcolor()
    end

    -- Create indices
    indices = {}

    -- Inner lines
    for j = 0, 4 do
        -- Inner layers
        table.insert(indices, j)
        table.insert(indices, j+5)

        table.insert(indices, (j+1)%5)
        table.insert(indices, j+5)

        -- Layers themselves
        table.insert(indices, (j+1)%5)
        table.insert(indices, j)

        table.insert(indices, (j+1)%5+5)
        table.insert(indices, j+5)

        -- Connecting to top and bottom vertices
        table.insert(indices, j)
        table.insert(indices, 5+5)

        table.insert(indices, j+5)
        table.insert(indices, 5+5+1)
    end

    return { vertices, indices }
end
