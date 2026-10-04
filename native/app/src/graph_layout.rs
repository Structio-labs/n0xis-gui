// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! Layered layout and orthogonal edge routing for one function's control-flow
//! graph. Pure geometry over the engine's blocks and edges: nothing here decides
//! what an edge means, only where it is drawn. An edge that points back up the
//! drawing is routed around the side because of where its ends are, not because
//! it is called a loop; the engine's edge kinds are the only labels shown.

/// Space between blocks in one row.
pub const H_GAP: f32 = 36.;
/// The least height of the band between two rows, where edges run sideways.
pub const BAND: f32 = 40.;
/// Distance between parallel edge runs, sideways in a band or down a gutter.
pub const TRACK: f32 = 8.;
/// Distance from the blocks to the first edge run in a side gutter.
pub const GUTTER: f32 = 22.;
/// Empty space around the whole graph.
pub const MARGIN: f32 = 24.;

/// Sweeps of the barycentre ordering and of the position refinement.
const ORDER_SWEEPS: usize = 6;
const POSITION_SWEEPS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.w / 2.
    }

    /// Whether the point lies strictly inside, not on the border.
    #[cfg(test)]
    pub fn contains_strictly(&self, x: f32, y: f32) -> bool {
        x > self.x + 0.5 && x < self.right() - 0.5 && y > self.y + 0.5 && y < self.bottom() - 0.5
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeIn {
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// One rectangle per input node, in input order.
    pub nodes: Vec<Rect>,
    /// One polyline per input edge, in input order: it leaves the bottom of
    /// its source and enters the top of its target.
    pub edges: Vec<Vec<(f32, f32)>>,
    pub width: f32,
    pub height: f32,
}

/// Lay out `sizes` (width, height per node) joined by `edges`, with `entry` on
/// the top row.
pub fn layout(sizes: &[(f32, f32)], edges: &[EdgeIn], entry: usize) -> Layout {
    let n = sizes.len();
    if n == 0 {
        return Layout { nodes: Vec::new(), edges: vec![Vec::new(); edges.len()], width: 0., height: 0. };
    }
    let edges: Vec<EdgeIn> = edges.iter().copied().filter(|e| e.from < n && e.to < n).collect::<Vec<_>>();
    let backward = backward_edges(n, &edges, entry.min(n - 1));
    let rank = ranks(n, &edges, &backward);
    let rows = order_rows(n, &edges, &backward, &rank);
    let xs = positions(sizes, &edges, &backward, &rank, &rows);
    route(sizes, &edges, &backward, &rank, &rows, &xs)
}

/// Edges that go against a depth-first walk from the entry (and then from any
/// block it does not reach): taking them out leaves a graph with no cycle,
/// which is what rows can be stacked from.
fn backward_edges(n: usize, edges: &[EdgeIn], entry: usize) -> Vec<bool> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, e) in edges.iter().enumerate() {
        out[e.from].push(i);
    }
    // 0 unvisited, 1 on the stack, 2 done.
    let mut state = vec![0u8; n];
    let mut backward = vec![false; edges.len()];
    let starts = std::iter::once(entry).chain(0..n);
    for start in starts {
        if state[start] != 0 {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        state[start] = 1;
        while let Some(&mut (node, ref mut next)) = stack.last_mut() {
            if let Some(&ei) = out[node].get(*next) {
                *next += 1;
                let to = edges[ei].to;
                match state[to] {
                    0 => {
                        state[to] = 1;
                        stack.push((to, 0));
                    }
                    1 => backward[ei] = true,
                    _ => {}
                }
            } else {
                state[node] = 2;
                stack.pop();
            }
        }
    }
    backward
}

/// The row of each node: the longest path to it over forward edges.
fn ranks(n: usize, edges: &[EdgeIn], backward: &[bool]) -> Vec<usize> {
    let mut indegree = vec![0usize; n];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, e) in edges.iter().enumerate() {
        if !backward[i] {
            indegree[e.to] += 1;
            out[e.from].push(e.to);
        }
    }
    let mut rank = vec![0usize; n];
    let mut ready: Vec<usize> = (0..n).filter(|&v| indegree[v] == 0).rev().collect();
    while let Some(v) = ready.pop() {
        for &w in &out[v] {
            rank[w] = rank[w].max(rank[v] + 1);
            indegree[w] -= 1;
            if indegree[w] == 0 {
                ready.push(w);
            }
        }
    }
    rank
}

/// The nodes of each row, left to right: address order first, then sorted a few
/// times by the mean position of their neighbours to cut crossings.
fn order_rows(n: usize, edges: &[EdgeIn], backward: &[bool], rank: &[usize]) -> Vec<Vec<usize>> {
    let rows_count = rank.iter().copied().max().unwrap_or(0) + 1;
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); rows_count];
    for v in 0..n {
        rows[rank[v]].push(v);
    }
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, e) in edges.iter().enumerate() {
        if !backward[i] {
            preds[e.to].push(e.from);
            succs[e.from].push(e.to);
        }
    }
    let mut pos = vec![0f32; n];
    let refresh = |rows: &Vec<Vec<usize>>, pos: &mut Vec<f32>| {
        for row in rows {
            for (i, &v) in row.iter().enumerate() {
                pos[v] = i as f32 + 0.5 - row.len() as f32 / 2.;
            }
        }
    };
    refresh(&rows, &mut pos);
    for sweep in 0..ORDER_SWEEPS {
        let down = sweep % 2 == 0;
        let order: Vec<usize> = if down { (1..rows_count).collect() } else { (0..rows_count.saturating_sub(1)).rev().collect() };
        for r in order {
            let neighbours = if down { &preds } else { &succs };
            let mut keyed: Vec<(f32, usize, usize)> = rows[r]
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let ns = &neighbours[v];
                    let key = if ns.is_empty() { pos[v] } else { ns.iter().map(|&u| pos[u]).sum::<f32>() / ns.len() as f32 };
                    (key, i, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            rows[r] = keyed.into_iter().map(|(_, _, v)| v).collect();
            for (i, &v) in rows[r].iter().enumerate() {
                pos[v] = i as f32 + 0.5 - rows[r].len() as f32 / 2.;
            }
        }
    }
    rows
}

/// Left edge of each node. Rows keep their order and spacing; within that,
/// each node moves toward the centre of its neighbours.
fn positions(sizes: &[(f32, f32)], edges: &[EdgeIn], backward: &[bool], _rank: &[usize], rows: &[Vec<usize>]) -> Vec<f32> {
    let n = sizes.len();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, e) in edges.iter().enumerate() {
        if !backward[i] && e.from != e.to {
            preds[e.to].push(e.from);
            succs[e.from].push(e.to);
        }
    }
    let mut x = vec![0f32; n];
    // Packed and centred on zero to start with.
    for row in rows {
        let total: f32 = row.iter().map(|&v| sizes[v].0).sum::<f32>() + H_GAP * row.len().saturating_sub(1) as f32;
        let mut at = -total / 2.;
        for &v in row {
            x[v] = at;
            at += sizes[v].0 + H_GAP;
        }
    }
    let center = |x: &[f32], v: usize| x[v] + sizes[v].0 / 2.;
    for sweep in 0..POSITION_SWEEPS {
        let down = sweep % 2 == 0;
        let order: Vec<usize> = if down { (1..rows.len()).collect() } else { (0..rows.len().saturating_sub(1)).rev().collect() };
        for r in order {
            let row = &rows[r];
            if row.is_empty() {
                continue;
            }
            let neighbours = if down { &preds } else { &succs };
            let desired: Vec<f32> = row
                .iter()
                .map(|&v| {
                    let ns = &neighbours[v];
                    if ns.is_empty() {
                        x[v]
                    } else {
                        ns.iter().map(|&u| center(&x, u)).sum::<f32>() / ns.len() as f32 - sizes[v].0 / 2.
                    }
                })
                .collect();
            // Push right from the left, push left from the right, take the mean:
            // both respect the spacing, so their mean does too.
            let mut from_left = desired.clone();
            for i in 1..row.len() {
                let least = from_left[i - 1] + sizes[row[i - 1]].0 + H_GAP;
                from_left[i] = from_left[i].max(least);
            }
            let mut from_right = desired.clone();
            for i in (0..row.len().saturating_sub(1)).rev() {
                let most = from_right[i + 1] - sizes[row[i]].0 - H_GAP;
                from_right[i] = from_right[i].min(most);
            }
            for (i, &v) in row.iter().enumerate() {
                x[v] = (from_left[i] + from_right[i]) / 2.;
            }
        }
    }
    let min_x = x.iter().copied().fold(f32::INFINITY, f32::min);
    for value in &mut x {
        *value += MARGIN - min_x;
    }
    x
}

/// How an edge travels.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Path {
    /// To the next row: down, across in the band between, down again.
    Adjacent,
    /// Down past rows in between, along a gutter clear of their blocks.
    Long { gutter: f32 },
    /// Back up (or to its own row): out to the right of every block it passes.
    Around { gutter: f32 },
}

/// A sideways run of an edge in one band, before tracks are known.
struct Run {
    edge: usize,
    band: usize,
    from_x: f32,
    to_x: f32,
}

fn route(sizes: &[(f32, f32)], edges: &[EdgeIn], backward: &[bool], rank: &[usize], rows: &[Vec<usize>], xs: &[f32]) -> Layout {
    let n = sizes.len();
    let rows_count = rows.len();
    let left = |v: usize| xs[v];
    let right = |v: usize| xs[v] + sizes[v].0;

    // Ports: a node's outgoing edges leave its bottom spread across it, sorted by
    // where they head; incoming edges arrive across its top the same way.
    let heading = |e: &EdgeIn, i: usize| -> f32 {
        if backward[i] || rank[e.to] <= rank[e.from] { f32::INFINITY } else { xs[e.to] + sizes[e.to].0 / 2. }
    };
    let mut out_port = vec![0f32; edges.len()];
    let mut in_port = vec![0f32; edges.len()];
    for v in 0..n {
        let mut outs: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].from == v).collect();
        outs.sort_by(|&a, &b| heading(&edges[a], a).total_cmp(&heading(&edges[b], b)).then(a.cmp(&b)));
        for (k, &i) in outs.iter().enumerate() {
            out_port[i] = xs[v] + sizes[v].0 * (k as f32 + 1.) / (outs.len() as f32 + 1.);
        }
        let mut ins: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].to == v).collect();
        let source_x = |i: usize| -> f32 {
            let e = &edges[i];
            if backward[i] || rank[e.to] <= rank[e.from] { f32::INFINITY } else { xs[e.from] + sizes[e.from].0 / 2. }
        };
        ins.sort_by(|&a, &b| source_x(a).total_cmp(&source_x(b)).then(a.cmp(&b)));
        for (k, &i) in ins.iter().enumerate() {
            in_port[i] = xs[v] + sizes[v].0 * (k as f32 + 1.) / (ins.len() as f32 + 1.);
        }
    }

    // The extent of each row, for gutters.
    let row_span = |r: usize| -> (f32, f32) {
        rows[r].iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| (lo.min(left(v)), hi.max(right(v))))
    };

    // Choose each edge's path. Bands are numbered by the row below them: band r
    // lies above row r, and band `rows_count` below the last row.
    let mut paths = vec![Path::Adjacent; edges.len()];
    let mut right_lanes: Vec<(usize, usize, usize)> = Vec::new(); // (top row, bottom row, lane)
    let mut long_lanes: Vec<(f32, usize, usize)> = Vec::new(); // (gutter x, top row, bottom row)
    for (i, e) in edges.iter().enumerate() {
        let (ru, rv) = (rank[e.from], rank[e.to]);
        if !backward[i] && rv == ru + 1 {
            continue;
        }
        if !backward[i] && rv > ru + 1 {
            // A column clear of every block in the rows the edge passes.
            let target = (out_port[i] + in_port[i]) / 2.;
            let mut blocked: Vec<(f32, f32)> =
                (ru + 1..rv).flat_map(|r| rows[r].iter().map(|&v| (left(v) - TRACK, right(v) + TRACK))).collect();
            // Earlier long edges in overlapping rows already use their columns.
            for &(gx, top, bottom) in &long_lanes {
                if top < rv && ru < bottom {
                    blocked.push((gx - TRACK / 2., gx + TRACK / 2.));
                }
            }
            let gutter = clear_column(&blocked, target);
            long_lanes.push((gutter, ru, rv));
            paths[i] = Path::Long { gutter };
            continue;
        }
        // Up, or within one row: around the right of every row it passes.
        let (top, bottom) = (rv.min(ru), rv.max(ru));
        let lane = (0..).find(|&l| !right_lanes.iter().any(|&(t, b, lane)| lane == l && t <= bottom && top <= b)).unwrap_or(0);
        right_lanes.push((top, bottom, lane));
        let extent = (top..=bottom).map(|r| row_span(r).1).fold(f32::NEG_INFINITY, f32::max);
        paths[i] = Path::Around { gutter: extent + GUTTER + lane as f32 * TRACK };
    }

    // Sideways runs per band, then tracks per band.
    let mut runs: Vec<Run> = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let (ru, rv) = (rank[e.from], rank[e.to]);
        match paths[i] {
            Path::Adjacent => runs.push(Run { edge: i, band: rv, from_x: out_port[i], to_x: in_port[i] }),
            Path::Long { gutter } => {
                runs.push(Run { edge: i, band: ru + 1, from_x: out_port[i], to_x: gutter });
                runs.push(Run { edge: i, band: rv, from_x: gutter, to_x: in_port[i] });
            }
            Path::Around { gutter } => {
                runs.push(Run { edge: i, band: ru + 1, from_x: out_port[i], to_x: gutter });
                runs.push(Run { edge: i, band: rv, from_x: gutter, to_x: in_port[i] });
            }
        }
    }
    let mut track_of = vec![0usize; runs.len()];
    let mut tracks_in_band = vec![1usize; rows_count + 1];
    for (band, tracks) in tracks_in_band.iter_mut().enumerate() {
        let mut here: Vec<usize> = (0..runs.len()).filter(|&k| runs[k].band == band).collect();
        here.sort_by(|&a, &b| runs[a].from_x.min(runs[a].to_x).total_cmp(&runs[b].from_x.min(runs[b].to_x)));
        let mut track_ends: Vec<f32> = Vec::new();
        for k in here {
            let (lo, hi) = (runs[k].from_x.min(runs[k].to_x), runs[k].from_x.max(runs[k].to_x));
            let t = match track_ends.iter().position(|&end| end + TRACK <= lo) {
                Some(t) => t,
                None => {
                    track_ends.push(f32::NEG_INFINITY);
                    track_ends.len() - 1
                }
            };
            track_ends[t] = hi;
            track_of[k] = t;
        }
        *tracks = track_ends.len().max(1);
    }

    // Rows top to bottom: each band as tall as its tracks need.
    let band_height = |b: usize| BAND + (tracks_in_band[b] - 1) as f32 * TRACK;
    let row_height: Vec<f32> = rows.iter().map(|row| row.iter().map(|&v| sizes[v].1).fold(0., f32::max)).collect();
    let mut row_top = vec![0f32; rows_count];
    let mut band_top = vec![0f32; rows_count + 1];
    let mut y = MARGIN;
    for r in 0..rows_count {
        band_top[r] = y;
        y += band_height(r);
        row_top[r] = y;
        y += row_height[r];
    }
    band_top[rows_count] = y;
    let height = y + band_height(rows_count) + MARGIN;
    let track_y = |band: usize, track: usize| band_top[band] + BAND / 2. + track as f32 * TRACK;

    let nodes: Vec<Rect> = (0..n).map(|v| Rect { x: xs[v], y: row_top[rank[v]], w: sizes[v].0, h: sizes[v].1 }).collect();

    let mut polylines: Vec<Vec<(f32, f32)>> = vec![Vec::new(); edges.len()];
    let mut runs_of: Vec<Vec<usize>> = vec![Vec::new(); edges.len()];
    for (k, run) in runs.iter().enumerate() {
        runs_of[run.edge].push(k);
    }
    for (i, e) in edges.iter().enumerate() {
        let (src, dst) = (&nodes[e.from], &nodes[e.to]);
        let (sx, tx) = (out_port[i], in_port[i]);
        let mut pts = vec![(sx, src.bottom())];
        for &k in &runs_of[i] {
            let ty = track_y(runs[k].band, track_of[k]);
            pts.push((runs[k].from_x, ty));
            pts.push((runs[k].to_x, ty));
        }
        pts.push((tx, dst.y));
        polylines[i] = simplify(pts);
    }

    let width = nodes.iter().map(Rect::right).fold(0., f32::max).max(
        paths.iter().map(|p| match p {
            Path::Around { gutter } | Path::Long { gutter } => *gutter,
            Path::Adjacent => 0.,
        })
        .fold(0., f32::max),
    ) + MARGIN;
    Layout { nodes, edges: polylines, width, height }
}

/// The x nearest `target` that lies in none of the `blocked` intervals.
fn clear_column(blocked: &[(f32, f32)], target: f32) -> f32 {
    let inside = |x: f32| blocked.iter().any(|&(lo, hi)| x > lo && x < hi);
    if !inside(target) {
        return target;
    }
    let mut candidates: Vec<f32> = blocked.iter().flat_map(|&(lo, hi)| [lo, hi]).filter(|&x| !inside(x)).collect();
    candidates.sort_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()));
    candidates.first().copied().unwrap_or_else(|| blocked.iter().map(|b| b.1).fold(target, f32::max) + GUTTER)
}

/// Drop points that do not turn, so every remaining segment is one straight run.
fn simplify(points: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    for p in points {
        if out.last().is_some_and(|q| (q.0 - p.0).abs() < 0.01 && (q.1 - p.1).abs() < 0.01) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let collinear = ((a.0 - b.0).abs() < 0.01 && (b.0 - p.0).abs() < 0.01) || ((a.1 - b.1).abs() < 0.01 && (b.1 - p.1).abs() < 0.01);
            if collinear {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

/// A route for an edge whose ends were moved by hand: down from the source, across
/// halfway, down into the target; or around the right when the target is not
/// below. Used only after a block is dragged.
pub fn reroute(src: &Rect, sx: f32, dst: &Rect, tx: f32) -> Vec<(f32, f32)> {
    if dst.y > src.bottom() + BAND / 2. {
        let mid = (src.bottom() + dst.y) / 2.;
        return simplify(vec![(sx, src.bottom()), (sx, mid), (tx, mid), (tx, dst.y)]);
    }
    let gutter = src.right().max(dst.right()) + GUTTER;
    let below = src.bottom() + BAND / 2.;
    let above = dst.y - BAND / 2.;
    simplify(vec![(sx, src.bottom()), (sx, below), (gutter, below), (gutter, above), (tx, above), (tx, dst.y)])
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{EdgeIn, Layout, Rect, layout, reroute};

    fn e(from: usize, to: usize) -> EdgeIn {
        EdgeIn { from, to }
    }

    /// The properties every drawing must have, whatever the graph.
    fn check(sizes: &[(f32, f32)], edges: &[EdgeIn], l: &Layout) {
        // No two blocks overlap.
        for (i, a) in l.nodes.iter().enumerate() {
            for b in &l.nodes[i + 1..] {
                let apart = a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        for (i, edge) in edges.iter().enumerate() {
            let pts = &l.edges[i];
            assert!(pts.len() >= 2, "edge {i} has a path");
            let (src, dst) = (&l.nodes[edge.from], &l.nodes[edge.to]);
            let first = pts[0];
            let last = pts[pts.len() - 1];
            assert!((first.1 - src.bottom()).abs() < 0.01 && first.0 >= src.x && first.0 <= src.right(), "edge {i} leaves the bottom of its source");
            assert!((last.1 - dst.y).abs() < 0.01 && last.0 >= dst.x && last.0 <= dst.right(), "edge {i} enters the top of its target");
            for w in pts.windows(2) {
                let ((x1, y1), (x2, y2)) = (w[0], w[1]);
                assert!((x1 - x2).abs() < 0.01 || (y1 - y2).abs() < 0.01, "edge {i}: segment {w:?} is not straight");
                // No segment passes through a block: sample along it.
                for s in 0..=20 {
                    let t = s as f32 / 20.;
                    let (x, y) = (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t);
                    for (v, r) in l.nodes.iter().enumerate() {
                        assert!(!r.contains_strictly(x, y), "edge {i} ({edge:?}) crosses block {v} at ({x}, {y})");
                    }
                }
            }
        }
        assert_eq!(l.nodes.len(), sizes.len());
    }

    #[test]
    fn a_diamond_stacks_in_three_rows() {
        let sizes = [(120., 40.); 4];
        let edges = [e(0, 1), e(0, 2), e(1, 3), e(2, 3)];
        let l = layout(&sizes, &edges, 0);
        check(&sizes, &edges, &l);
        assert!(l.nodes[0].bottom() < l.nodes[1].y && l.nodes[1].y == l.nodes[2].y && l.nodes[2].bottom() < l.nodes[3].y);
    }

    #[test]
    fn an_edge_back_up_goes_around_the_side() {
        let sizes = [(100., 30.), (140., 60.), (100., 30.), (80., 30.)];
        let edges = [e(0, 1), e(1, 2), e(2, 1), e(2, 3), e(1, 1)];
        let l = layout(&sizes, &edges, 0);
        check(&sizes, &edges, &l);
        let rightmost = l.nodes.iter().map(Rect::right).fold(0., f32::max);
        assert!(l.edges[2].iter().any(|&(x, _)| x > rightmost), "{:?}", l.edges[2]);
        assert!(l.edges[4].iter().any(|&(x, _)| x > rightmost), "the edge to itself too: {:?}", l.edges[4]);
    }

    #[test]
    fn an_edge_past_a_row_keeps_clear_of_its_blocks() {
        // 0 -> 1 -> 2 -> 3, and 0 -> 3 straight past rows 1 and 2.
        let sizes = [(200., 30.), (400., 30.), (400., 30.), (200., 30.)];
        let edges = [e(0, 1), e(1, 2), e(2, 3), e(0, 3)];
        let l = layout(&sizes, &edges, 0);
        check(&sizes, &edges, &l);
    }

    #[test]
    fn graphs_of_every_shape_keep_the_properties() {
        // A fixed pseudo-random sweep: the same graphs on every run.
        let mut seed: u64 = 0x5eed_1234;
        let mut next = |m: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % m
        };
        for _ in 0..60 {
            let n = 2 + next(24) as usize;
            let sizes: Vec<(f32, f32)> = (0..n).map(|_| (60. + next(200) as f32, 20. + next(120) as f32)).collect();
            let mut edges = Vec::new();
            for v in 0..n {
                // Mostly forward, like code; some back and some far.
                for _ in 0..next(3) {
                    let to = if next(5) == 0 { next(n as u64) as usize } else { (v + 1 + next(3) as usize).min(n - 1) };
                    edges.push(e(v, to));
                }
            }
            let l = layout(&sizes, &edges, 0);
            check(&sizes, &edges, &l);
        }
    }

    #[test]
    fn a_dragged_block_is_reached_with_straight_runs() {
        let a = Rect { x: 0., y: 0., w: 100., h: 40. };
        let b = Rect { x: 300., y: 200., w: 100., h: 40. };
        for pts in [reroute(&a, 50., &b, 350.), reroute(&b, 350., &a, 50.)] {
            for w in pts.windows(2) {
                assert!((w[0].0 - w[1].0).abs() < 0.01 || (w[0].1 - w[1].1).abs() < 0.01, "{pts:?}");
            }
        }
    }
}
