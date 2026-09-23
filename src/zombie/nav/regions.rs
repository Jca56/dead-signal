//! Which ground can get to which: the grid's floors gathered into regions
//! (ground a body can walk both ways between), and for each region every
//! other it can get to, walking and dropping. Worked out once, so asking
//! whether a way exists costs nothing, and a search is never started that
//! can only fail (the costliest search of all, the whole grid looked over).

use super::{AROUND, NavGrid};

/// No region.
const NONE: u32 = u32::MAX;

#[derive(Default)]
pub struct Regions {
    /// Each floor's region.
    of: Vec<u32>,
    /// For each region, the regions it can get to (itself too), a bit each.
    reach: Vec<Vec<u64>>,
}

impl Regions {
    pub fn build(grid: &NavGrid) -> Self {
        let nodes = grid.height.len();
        // Walked both ways, not dropped: the same region.
        let mut parent: Vec<usize> = (0..nodes).collect();
        fn root(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        for i in 0..nodes {
            let a = i as u32;
            for k in 0..AROUND.len() {
                let Some(b) = grid.link(a, k) else { continue };
                let both_ways = grid.linked(a, b) && grid.linked(b, a) && !grid.drop_to(a, b) && !grid.drop_to(b, a);
                if both_ways {
                    let (ra, rb) = (root(&mut parent, i), root(&mut parent, b as usize));
                    parent[ra] = rb;
                }
            }
        }
        let mut of = vec![NONE; nodes];
        let mut ids = std::collections::HashMap::new();
        for (i, region) in of.iter_mut().enumerate() {
            let r = root(&mut parent, i);
            let next = ids.len() as u32;
            *region = *ids.entry(r).or_insert(next);
        }
        let count = ids.len();
        // One region leads to another where any way (a drop too) crosses
        // between them.
        let mut next: Vec<Vec<u32>> = vec![Vec::new(); count];
        for i in 0..nodes {
            let a = i as u32;
            for k in 0..AROUND.len() {
                let Some(b) = grid.link(a, k) else { continue };
                if of[b as usize] != of[i] && grid.linked(a, b) {
                    next[of[i] as usize].push(of[b as usize]);
                }
            }
        }
        for n in &mut next {
            n.sort_unstable();
            n.dedup();
        }
        let words = count.div_ceil(64);
        let reach = (0..count)
            .map(|r| {
                let mut seen = vec![0u64; words];
                let mut todo = vec![r as u32];
                seen[r / 64] |= 1 << (r % 64);
                while let Some(at) = todo.pop() {
                    for &n in &next[at as usize] {
                        let (w, bit) = (n as usize / 64, 1u64 << (n % 64));
                        if seen[w] & bit == 0 {
                            seen[w] |= bit;
                            todo.push(n);
                        }
                    }
                }
                seen
            })
            .collect();
        Self { of, reach }
    }

    /// Whether a body on the floor at index `a` can get to the one at `b`.
    pub fn reaches(&self, a: usize, b: usize) -> bool {
        match (self.of.get(a), self.of.get(b)) {
            (Some(&ra), Some(&rb)) if ra != NONE && rb != NONE => self.reach[ra as usize][rb as usize / 64] & (1 << (rb % 64)) != 0,
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.reach.len()
    }
}
