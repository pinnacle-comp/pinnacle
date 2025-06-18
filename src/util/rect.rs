use smithay::utils::{Logical, Point, Rectangle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Returns the indices of rectangles in `rects` that are closest to `rect`
/// in the given direction.
pub fn closest_in_dir(
    rect: Rectangle<i32, Logical>,
    rects: &[Rectangle<i32, Logical>],
    dir: Direction,
) -> Vec<usize> {
    let (mut inside, mut overlap, mut enclosing, mut no_overlap) = rects
        .iter()
        .enumerate()
        .filter(|(_, other)| rect != **other)
        .filter(|(_, other)| match dir {
            Direction::Left => other.center().x < rect.center().x,
            Direction::Right => other.center().x > rect.center().x,
            Direction::Up => other.center().y < rect.center().y,
            Direction::Down => other.center().y > rect.center().y,
        })
        .fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |mut acc, (i, &other)| {
                let (inside, overlap, enclosing, no_overlap) = &mut acc;

                if rect.contains_rect(other) {
                    inside.push((i, other));
                } else if other.contains_rect(rect) {
                    enclosing.push((i, other));
                } else if rect.overlaps(other) {
                    overlap.push((i, other));
                } else {
                    no_overlap.push((i, other));
                }

                acc
            },
        );

    inside.retain(|&(_, other)| is_more_in_direction(rect, other, dir));
    inside.sort_by(|(_, a), (_, b)| {
        distance(rect.center(), a.center()).total_cmp(&distance(rect.center(), b.center()))
    });

    overlap.retain(|&(_, other)| {
        let center_in_rect = rect.contains(other.center());
        if center_in_rect {
            is_more_in_direction(rect, other, dir)
        } else {
            let spans_side = match dir {
                Direction::Left | Direction::Right => {
                    other.bottom() >= rect.bottom() && other.top() <= rect.top()
                }
                Direction::Up | Direction::Down => {
                    other.left() <= rect.left() && other.right() >= rect.right()
                }
            };

            let center_is_to_side = match dir {
                Direction::Left | Direction::Right => {
                    other.center().y >= rect.top() && other.center().y <= rect.bottom()
                }
                Direction::Up | Direction::Down => {
                    other.center().x >= rect.left() && other.center().x <= rect.right()
                }
            };

            let corner_enough = is_more_in_direction(rect, other, dir);

            spans_side || center_is_to_side || corner_enough
        }
    });
    overlap.sort_by(|(_, a), (_, b)| {
        distance(rect.center(), a.center()).total_cmp(&distance(rect.center(), b.center()))
    });

    enclosing.retain(|&(_, other)| is_more_in_direction(rect, other, dir));
    enclosing.sort_by(|(_, a), (_, b)| {
        distance(rect.center(), a.center()).total_cmp(&distance(rect.center(), b.center()))
    });

    no_overlap.retain(|&(_, other)| {
        let spans_side = match dir {
            Direction::Left | Direction::Right => {
                other.bottom() >= rect.bottom() && other.top() <= rect.top()
            }
            Direction::Up | Direction::Down => {
                other.left() <= rect.left() && other.right() >= rect.right()
            }
        };

        let center_is_to_side = match dir {
            Direction::Left | Direction::Right => {
                other.center().y >= rect.top() && other.center().y <= rect.bottom()
            }
            Direction::Up | Direction::Down => {
                other.center().x >= rect.left() && other.center().x <= rect.right()
            }
        };

        let past_edge = match dir {
            Direction::Left => other.right() < rect.left(),
            Direction::Right => other.left() > rect.right(),
            Direction::Up => other.bottom() < rect.top(),
            Direction::Down => other.top() > rect.bottom(),
        };

        let corner_enough = is_more_in_direction(rect, other, dir);

        spans_side || center_is_to_side || (past_edge && corner_enough)
    });
    no_overlap
        .sort_by(|(_, a), (_, b)| distance(rect.loc, a.loc).total_cmp(&distance(rect.loc, b.loc)));

    inside
        .into_iter()
        .chain(overlap)
        .chain(enclosing)
        .chain(no_overlap)
        .map(|(idx, _)| idx)
        .collect()
}

/// Returns whether the `other` rectangle's center is more left/rightward or up/downward
/// from `rect`'s center based on a 45 degree cross centered on `rect`.
///
/// This does not distinguish directions on the same axis. Check where the center of `other`
/// is beforehand to know that information.
///
/// |-----|
/// |\ U /|
/// | \ / |
/// |L x R|
/// | / \ |
/// |/ D \|
/// |-----|
fn is_more_in_direction(
    rect: Rectangle<i32, Logical>,
    other: Rectangle<i32, Logical>,
    dir: Direction,
) -> bool {
    let aux_rect = Rectangle::bounding_box([other.center(), rect.center()]);

    match dir {
        Direction::Left | Direction::Right => aux_rect.aspect_ratio() >= 1.0,
        Direction::Up | Direction::Down => aux_rect.aspect_ratio() <= 1.0,
    }
}

fn distance(point1: Point<i32, Logical>, point2: Point<i32, Logical>) -> f32 {
    let a = (point2.x - point1.x).abs() as f32;
    let b = (point2.y - point1.y).abs() as f32;
    f32::hypot(a, b)
}

trait RectExt {
    fn top(self) -> i32;
    fn bottom(self) -> i32;
    fn left(self) -> i32;
    fn right(self) -> i32;
    fn center(self) -> Point<i32, Logical>;
    fn aspect_ratio(self) -> f32;
}

impl RectExt for Rectangle<i32, Logical> {
    fn top(self) -> i32 {
        self.loc.y
    }

    fn bottom(self) -> i32 {
        self.loc.y + self.size.h
    }

    fn left(self) -> i32 {
        self.loc.x
    }

    fn right(self) -> i32 {
        self.loc.x + self.size.w
    }

    fn center(self) -> Point<i32, Logical> {
        Point::new(self.loc.x + self.size.w / 2, self.loc.y + self.size.h / 2)
    }

    fn aspect_ratio(self) -> f32 {
        self.size.w as f32 / self.size.h as f32
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::{prelude::Strategy, proptest};

    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    /// ┌───┬───┐
    /// │ 0 │ 1 │
    /// ├───┼───┤
    /// │ 2 │ 3 │
    /// └───┴───┘
    #[test]
    fn grid_works() {
        let rects = [
            rect(0, 0, 50, 50),
            rect(50, 0, 50, 50),
            rect(0, 50, 50, 50),
            rect(50, 50, 50, 50),
        ];

        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Left), vec![]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Right), vec![1]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Down), vec![2]);

        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Left), vec![0]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Right), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Down), vec![3]);

        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Left), vec![]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Right), vec![3]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Up), vec![0]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Down), vec![]);

        assert_eq!(closest_in_dir(rects[3], &rects, Direction::Left), vec![2]);
        assert_eq!(closest_in_dir(rects[3], &rects, Direction::Right), vec![]);
        assert_eq!(closest_in_dir(rects[3], &rects, Direction::Up), vec![1]);
        assert_eq!(closest_in_dir(rects[3], &rects, Direction::Down), vec![]);
    }

    /// ┌───┬───┐
    /// │   │ 1 │
    /// │ 0 ├───┤
    /// │   │ 2 │
    /// └───┴───┘
    #[test]
    fn master_stack_works() {
        let rects = [
            rect(0, 0, 50, 100),
            rect(50, 0, 50, 50),
            rect(50, 50, 50, 50),
        ];

        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Left), vec![]);
        assert_eq!(
            closest_in_dir(rects[0], &rects, Direction::Right),
            vec![1, 2]
        );
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Down), vec![]);

        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Left), vec![0]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Right), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Down), vec![2]);

        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Left), vec![0]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Right), vec![]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Up), vec![1]);
        assert_eq!(closest_in_dir(rects[2], &rects, Direction::Down), vec![]);
    }

    /// ┌────────────┐
    /// │       ┌───┐│
    /// │   0   │ 1 ││
    /// │       └───┘│
    /// └────────────┘
    #[test]
    fn one_window_inside_another_works() {
        let rects = [rect(0, 0, 200, 100), rect(125, 25, 50, 50)];

        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Left), vec![]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Right), vec![1]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[0], &rects, Direction::Down), vec![]);

        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Left), vec![0]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Right), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Up), vec![]);
        assert_eq!(closest_in_dir(rects[1], &rects, Direction::Down), vec![]);
    }

    #[allow(dead_code)]
    fn arbitrary_rect() -> impl Strategy<Value = Rectangle<i32, Logical>> {
        (-500i32..500, -500i32..500, 10i32..100, 10i32..100)
            .prop_map(|(x, y, w, h)| Rectangle::new((x, y).into(), (w, h).into()))
    }

    #[allow(dead_code)]
    #[derive(Debug, PartialEq, Eq)]
    struct HashRect(Rectangle<i32, Logical>);

    impl std::hash::Hash for HashRect {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.0.loc.x.hash(state);
            self.0.loc.y.hash(state);
            self.0.size.w.hash(state);
            self.0.size.h.hash(state);
        }
    }

    proptest! {
        // #[test]
        // #[ignore]
        // It turns out this problem is very hard.
        // The current algorithm may prevent some windows
        // from being navigable in certain instances.
        fn all_rects_are_reachable_by_direction(
            // We ignore duplicate rects as there's no good way to navigate them
            rects in proptest::collection::hash_set(arbitrary_rect().prop_map(HashRect), 2..100)
        ) {
            let rects = rects.into_iter().map(|rect| rect.0).collect::<Vec<_>>();
            println!("rects are {rects:?}");
            let all_idxs = (0..rects.len()).collect::<HashSet<_>>();
            let mut reachable_idxs = HashSet::new();
            for &rect in rects.iter() {
                println!("rect is {rect:?}");
                reachable_idxs.extend(closest_in_dir(rect, &rects, Direction::Left));
                reachable_idxs.extend(closest_in_dir(rect, &rects, Direction::Right));
                reachable_idxs.extend(closest_in_dir(rect, &rects, Direction::Up));
                reachable_idxs.extend(closest_in_dir(rect, &rects, Direction::Down));
            }

            let diff = all_idxs.difference(&reachable_idxs);
            let diffs = diff.collect::<Vec<_>>();
            println!("missing {diffs:?}");
            println!("missing {:?}", diffs.first().map(|thing| {
                rects[**thing]
            }));
            assert_eq!(reachable_idxs, all_idxs);
        }
    }
}
