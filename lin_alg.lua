identity = {
    {1., 0., 0., 0.},
    {0., 1., 0., 0.},
    {0., 0., 1., 0.},
    {0., 0., 0., 1.},
}

function gemm(a, b)
    out = {}
    for i=1, 4 do
        row = {}
        for j=1, 4 do
            m = 0.0
            for k=1, 4 do
                m = m + a[i][k] * b[k][j] 
            end
            row[j] = m
        end
        out[i] = row
    end
    return out 
end

function translate(x, y, z) 
    return {
        {1., 0., 0., 0.},
        {0., 1., 0., 0.},
        {0., 0., 1., 0.},
        {x, y, z, 1.},
    }
end

function cannon(matrix)
    q = 1
    out = {}
    for i = 1,4 do
        for j = 1,4 do
            out[q] = matrix[i][j]
            q = q + 1
        end
    end
    return out
end

function rot_y(angle)
    return {
        {math.cos(angle), 0., -math.sin(angle), 0.},
        {0., 1., 0., 0.},
        {math.sin(angle), 0., math.cos(angle), 0.},
        {0., 0., 0., 1.},
    }
end

function print_mat(matrix)
    print()
    for row = 1,4 do
        for col = 1,4 do
            io.write(matrix[col][row])
            io.write(" ")
        end
        print()
    end
end

function table.clone(org)
  return {table.unpack(org)}
end
