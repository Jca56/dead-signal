"""What every character built from tubes shares: a bmesh that carries a
colour per face and bone weights per vertex, rings of points round a
limb, tubes joining them (refusing a corkscrew), and a few helpers."""

import math

import bmesh
from mathutils import Matrix, Vector

SIDES = 8


def norm(v):
    return v.normalized()


def rotate(v, axis, degrees):
    return Matrix.Rotation(math.radians(degrees), 3, axis) @ v


class Builder:
    """A bmesh with a colour per face and bone weights per vertex."""

    def __init__(self):
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")
        self.deform = self.bm.verts.layers.deform.verify()
        self.groups = []

    def group(self, name):
        if name not in self.groups:
            self.groups.append(name)
        return self.groups.index(name)

    def vert(self, co, weights):
        v = self.bm.verts.new(co)
        for name, w in weights.items():
            v[self.deform][self.group(name)] = w
        return v

    def face(self, verts, colour):
        f = self.bm.faces.new(verts)
        for loop in f.loops:
            loop[self.col] = (*colour, 1.0)
        return f

    def ring(self, centre, axis, a_dir, a, b, weights, spin=0.0):
        """`SIDES` points round `centre`, perpendicular to `axis`: an
        ellipse `a` across `a_dir` and `b` across the third direction."""
        u = norm(a_dir - axis * a_dir.dot(axis))
        w = axis.cross(u)
        pts = []
        for k in range(SIDES):
            t = spin + k / SIDES * math.tau
            pts.append(self.vert(centre + u * (math.cos(t) * a) + w * (math.sin(t) * b), weights))
        return pts

    def tube(self, rings, colours, cap_start=True, cap_end=True):
        """Join rings with quads; `colours[i]` paints the band after ring i.
        Refuses rings whose first points are turned apart: a corkscrew."""
        for i in range(len(rings) - 1):
            r0, r1 = rings[i], rings[i + 1]
            twist = twist_degrees(r0, r1)
            if twist > 30.0:
                raise ValueError(f"rings {i} and {i + 1} twist {twist:.0f} degrees")
            for k in range(SIDES):
                n = (k + 1) % SIDES
                c = colours[i]
                if isinstance(c, list):
                    c = c[k]
                self.face((r0[k], r0[n], r1[n], r1[k]), c)
        if cap_start:
            self.face(list(reversed(rings[0])), colours[0] if not isinstance(colours[0], list) else colours[0][0])
        if cap_end:
            last = colours[-1] if not isinstance(colours[-1], list) else colours[-1][0]
            self.face(rings[-1], last)


def twist_degrees(r0, r1):
    """How far point 0 of one ring is turned from point 0 of the next, seen
    down the tube between them."""
    c0 = sum((v.co for v in r0), Vector()) / len(r0)
    c1 = sum((v.co for v in r1), Vector()) / len(r1)
    axis = c1 - c0
    if axis.length < 1e-6:
        return 0.0
    axis.normalize()
    u0 = r0[0].co - c0
    u1 = r1[0].co - c1
    u0 -= axis * u0.dot(axis)
    u1 -= axis * u1.dot(axis)
    if u0.length < 1e-6 or u1.length < 1e-6:
        return 0.0
    return math.degrees(u0.angle(u1))


def banded(a, b):
    """Alternate two shades round a tube, so the facets read as cloth folds."""
    return [a if k % 2 == 0 else b for k in range(SIDES)]
