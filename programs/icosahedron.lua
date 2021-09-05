function icosahedron(length, primitive)
    if primitive == "tris" then
        indices = icosohedron_triangle_indices()
    else
        indices = icosohedron_line_indices()
    end

    return {
        icosahedron_verts(length),
        indices,
    }
end

function icosahedron_verts(length)
    -- Edge length
    local l = length;

    -- Color
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

            local color
            if i == 0 then color = { 0, 1, 0 }
                elseif i == 1 then color = { 1, 0, 0 }
                elseif i == 2 then color = { 0, 0, 1 }
                else color = { 1, 0, 1 } end

            for k = 1, #(color) do
                table.insert(vertices, color[k])
            end
        end
    end

    -- Top and bottom vertices 
    local height = h2/2. + h1
    local top_bot = {
        { 0.0, height, 0.0, 1.0, 1.0, 1.0 },
        { 0.0, -height, 0.0, 0.01, 0.01, 0.01 },
    }

    for j = 1, 2 do
        local pos = top_bot[j]
        for k = 1, #pos do
            table.insert(vertices, pos[k])
        end
    end

    return vertices
end

function icosohedron_line_indices()
    -- Create indices
    local indices = {}

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

    return indices
end

function icosohedron_triangle_indices()
    -- Create indices
    local indices = {}

    -- Inner lines
    for j = 0, 4 do
        -- Inner layers
        table.insert(indices, (j+1)%5)
        table.insert(indices, j+5)
        table.insert(indices, j)

        table.insert(indices, (j+1)%5)
        table.insert(indices, (j+1)%5+5)
        table.insert(indices, j+5)

        -- Layers themselves
        table.insert(indices, (j+1)%5)
        table.insert(indices, j)
        table.insert(indices, 5+5)

        -- Connecting to top and bottom vertices
        table.insert(indices, j+5)
        table.insert(indices, (j+1)%5+5)
        table.insert(indices, 5+5+1)
    end

    return indices
end
