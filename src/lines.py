import itertools
import json

SIZE = 4
DIMS = 4

def reflect(coord, dim):
    l = list(coord)
    l[dim] = SIZE - 1 - l[dim]
    return tuple(l)

def pack(c, l, r, o):
    return (c << 6) + (l << 4) + (r << 2) + o

lines = set()
for independently_varying_dimensions in itertools.product(*((False, True) for _ in range(DIMS))):
    if all(independently_varying_dimensions): continue

    rmap = { dim_idx: product_tuple_idx for product_tuple_idx, dim_idx in enumerate([ dim for dim, varies in enumerate(independently_varying_dimensions) if varies ]) }
    dimset_lines = []

    for varying in itertools.product(*(range(SIZE) for _ in range(independently_varying_dimensions.count(True)))):
        dimset_lines.append(tuple([ [ varying[rmap[d]] if independently_varying_dimensions[d] else shared_coord for d in range(DIMS) ] for shared_coord in range(SIZE) ]))

    outputs = set()
    fix_dims = [ i for i, d in enumerate(independently_varying_dimensions) if not d ]
    for do_reflect in itertools.product(*((False, True) for _ in range(len(fix_dims)))):
        reflect_dims = [ i for e, i in zip(do_reflect, fix_dims) if e ]
        def apply_reflection(coord):
            for dim in reflect_dims:
                coord = reflect(coord, dim)
            return tuple(coord)
        r = (tuple([ apply_reflection(c) for c in line ]) for line in dimset_lines)
        outputs.update(r)
    lines.update(map(tuple, map(sorted, outputs)))
lines_packed = [ [ pack(*coord) for coord in line ] for line in lines ]
print(json.dumps(lines_packed))