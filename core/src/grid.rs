//! Small vector helpers and a dense scalar grid.

pub type V3 = [f32; 3];

#[inline]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[inline]
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
#[inline]
pub fn normalize(a: V3) -> V3 {
    let l = length(a);
    if l > 1e-12 {
        scale(a, 1.0 / l)
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Dense grid of scalar samples at nodes `origin + h * (i, j, k)`.
/// Memory layout: x fastest, then y, then z.
pub struct Grid {
    pub origin: V3,
    pub h: f32,
    pub n: [usize; 3],
    pub data: Vec<f32>,
}

impl Grid {
    pub fn new(origin: V3, h: f32, n: [usize; 3], fill: f32) -> Self {
        Grid {
            origin,
            h,
            n,
            data: vec![fill; n[0] * n[1] * n[2]],
        }
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        i + self.n[0] * (j + self.n[1] * k)
    }

    #[inline]
    pub fn pos(&self, i: usize, j: usize, k: usize) -> V3 {
        [
            self.origin[0] + self.h * i as f32,
            self.origin[1] + self.h * j as f32,
            self.origin[2] + self.h * k as f32,
        ]
    }

    /// Index range of nodes whose coordinate on `axis` lies in [lo, hi].
    pub fn range(&self, axis: usize, lo: f32, hi: f32) -> Option<(usize, usize)> {
        let a = ((lo - self.origin[axis]) / self.h).ceil();
        let b = ((hi - self.origin[axis]) / self.h).floor();
        let a = a.max(0.0) as i64;
        let b = b.min(self.n[axis] as f32 - 1.0) as i64;
        if b < a {
            None
        } else {
            Some((a as usize, b as usize))
        }
    }
}
