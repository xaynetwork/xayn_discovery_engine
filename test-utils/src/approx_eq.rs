// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{collections::BTreeSet, iter};

use float_cmp::ApproxEq;
use ndarray::{ArrayBase, Data, Dimension, IntoDimension, Ix};

/// Compares two "things" with approximate equality.
///
/// # Examples
///
/// This can be used to compare two floating point numbers:
///
/// ```
/// use xayn_test_utils::assert_approx_eq;
/// assert_approx_eq!(f32, 0.150_391_55, 0.150_391_6, ulps = 3);
/// ```
///
/// Or containers of such:
///
/// ```
/// use xayn_test_utils::assert_approx_eq;
/// assert_approx_eq!(f32, &[[1., 2.], [3., 4.]], vec![[1., 2.], [3., 4.]])
/// ```
///
/// Or ndarray arrays:
///
/// ```
/// use ndarray::arr2;
/// use xayn_test_utils::assert_approx_eq;
/// assert_approx_eq!(
///     f32,
///     arr2(&[[1., 2.], [3., 4.]]),
///     arr2(&[[1., 2.], [3., 4.]]),
/// );
/// ```
///
/// The number of `ulps` defaults to `2` if not specified.
///
/// # NaN Handling
///
/// The assertions treats two NaN values to be "approximately" equal.
///
/// While there are good reasons for two NaN values not to compare as equal in
/// general, they don't really apply for this assertions which tries to check if
/// something has "an expected outcome" instead of "two values being semantically
/// the same".
#[macro_export]
macro_rules! assert_approx_eq {
    ($t:ty, $left:expr, $right:expr $(,)?) => {
        $crate::assert_approx_eq!($t, $left, $right, epsilon = 0., ulps = 2)
    };
    ($t:ty, $left:expr, $right:expr, ulps = $ulps:expr $(,)?) => {
       $crate::assert_approx_eq!($t, $left, $right, epsilon = 0., ulps = $ulps)
    };
    ($t:ty, $left:expr, $right:expr, epsilon = $epsilon:expr $(,)?) => {
       $crate::assert_approx_eq!($t, $left, $right, epsilon = $epsilon, ulps = 2)
    };
    ($t:ty, $left:expr, $right:expr, epsilon = $epsilon:expr, ulps = $ulps:expr $(,)?) => {{
        let epsilon = $epsilon;
        let ulps = $ulps;
        let left = &$left;
        let right = &$right;
        let mut left_iter =
            $crate::ApproxEqIter::<$t>::indexed_iter_logical_order(left, Vec::new());
        let mut right_iter =
            $crate::ApproxEqIter::<$t>::indexed_iter_logical_order(right, Vec::new());
        loop {
            match (left_iter.next(), right_iter.next()) {
                (Some((lidx, lv)), Some((ridx, rv))) => {
                    std::assert_eq!(
                        lidx, ridx,
                        "Dimensionality mismatch when iterating in logical order: {:?} != {:?}",
                        lidx, ridx,
                    );
                    if !(lv.is_nan() && rv.is_nan()) {
                        std::assert!(
                            $crate::approx_eq!($t, lv, rv, ulps = ulps, epsilon = epsilon),
                            "Approximated equal assertion failed (ulps={:?}, epsilon={:?}) at index {:?}: {:?} != {:?}",
                            ulps, epsilon, lidx, lv, rv,
                        );
                    }
                }
                (Some(pair), None) => {
                    std::panic!("Left input is longer starting from index {:?}", pair);
                }
                (None, Some(pair)) => {
                    std::panic!("Right input is longer starting from index {:?}", pair);
                }
                (None, None) => break,
            }
        }
    }};
}

/// Helper trait for the [`assert_approx_eq!`] macro, only use it for that.
///
/// This can be implemented for both containers and leaf values (e.g. f32).
pub trait ApproxEqIter<'a, LeafElement>
where
    Self: 'a,
    LeafElement: ApproxEq + Copy,
{
    /// Flattened iterates over all leaf elements in this instance.
    ///
    /// The passed in `index_prefix` is the "index" at which
    /// this instance is placed.
    ///
    /// Leaf values implementing this should just return a iterator
    /// which yields a single tuple of their value and the
    /// passed in index prefix.
    ///
    /// Sequential containers are supposed to yield a tuple for each
    /// element in them in which the index is created by pushing
    /// the elements index in this container onto the `index_prefix`.
    fn indexed_iter_logical_order(
        &'a self,
        index_prefix: Vec<Ix>,
    ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, LeafElement)>>;
}

macro_rules! impl_approx_eq_iter {
    ($($t:ty),+ $(,)?) => {
        $(
            impl<'a> ApproxEqIter<'a, $t> for $t
            where
                $t: 'a + ApproxEq + Copy,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    Box::new(iter::once((index_prefix, *self)))
                }
            }

            impl<'a, T> ApproxEqIter<'a, $t> for &'a T
            where
                T: 'a + ApproxEqIter<'a, $t> + ?Sized,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    (*self).indexed_iter_logical_order(index_prefix)
                }
            }

            impl<'a, T> ApproxEqIter<'a, $t> for Option<T>
            where
                T: 'a + ApproxEqIter<'a, $t>,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    Box::new(self.iter().flat_map(move |el| {
                        let mut index_prefix = index_prefix.clone();
                        index_prefix.push(0);
                        el.indexed_iter_logical_order(index_prefix)
                    }))
                }
            }

            impl<'a, T> ApproxEqIter<'a, $t> for Vec<T>
            where
                T: 'a + ApproxEqIter<'a, $t>,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    self.as_slice().indexed_iter_logical_order(index_prefix)
                }
            }

            impl<'a, T, const N: usize> ApproxEqIter<'a, $t> for [T; N]
            where
                T: 'a + ApproxEqIter<'a, $t>,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    self.as_slice().indexed_iter_logical_order(index_prefix)
                }
            }

            impl<'a, T> ApproxEqIter<'a, $t> for [T]
            where
                T: 'a + ApproxEqIter<'a, $t>,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    Box::new(self.iter().enumerate().flat_map(move |(idx, el)| {
                        let mut index_prefix = index_prefix.clone();
                        index_prefix.push(idx);
                        el.indexed_iter_logical_order(index_prefix)
                    }))
                }
            }

            impl<'a, S, D> ApproxEqIter<'a, $t> for ArrayBase<S, D>
            where
                S: 'a + Data<Elem = $t>,
                D: 'a + Dimension,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    Box::new(self.indexed_iter().map(move |(idx, el)| {
                        let mut index_prefix = index_prefix.clone();
                        index_prefix.extend(idx.into_dimension().as_array_view().iter());
                        (index_prefix, *el)
                    }))
                }
            }

            impl<'a, T> ApproxEqIter<'a, $t> for BTreeSet<T>
            where
                T: 'a + ApproxEqIter<'a, $t>,
            {
                fn indexed_iter_logical_order(
                    &'a self,
                    index_prefix: Vec<Ix>,
                ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, $t)>> {
                    Box::new(self.iter().enumerate().flat_map(move |(idx, el)| {
                        let mut index_prefix = index_prefix.clone();
                        index_prefix.push(idx);
                        el.indexed_iter_logical_order(index_prefix)
                    }))
                }
            }
        )+
    };
}

impl_approx_eq_iter! { f32, f64 }

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use ndarray::{arr1, arr2, arr3};

    #[test]
    fn test_assert_approx_eq_float() {
        assert_approx_eq!(f32, 0.150_391_55, 0.150_391_6, ulps = 3);
        catch_unwind(|| assert_approx_eq!(f32, 0.150_391_55, 0.150_391_6, ulps = 2)).unwrap_err();
    }

    #[test]
    fn test_assert_approx_eq_iterable_1d() {
        assert_approx_eq!(f32, &[0.25, 1.25], &[0.25, 1.25]);
        assert_approx_eq!(f32, &[0.25, 1.25], arr1(&[0.25, 1.25]));
        assert_approx_eq!(f32, [0.25, 1.25], arr1(&[0.25, 1.25]));
    }

    #[test]
    #[should_panic(expected = "at index [1]")]
    fn test_assert_approx_eq_fails() {
        assert_approx_eq!(f32, &[0.35, 4.35], arr1(&[0.35, 4.45]));
    }

    #[test]
    #[should_panic(expected = "at index [0, 1, 2]")]
    fn test_assert_approx_eq_fails_multi_dimensional() {
        assert_approx_eq!(
            f32,
            &[[[0.25, 1.25, 0.], [0.0, 0.125, 0.]]],
            arr3(&[[[0.25, 1.25, 0.], [0.0, 0.125, 1.]]]),
        );
    }

    #[test]
    fn test_assert_approx_eq_iterable_nested() {
        assert_approx_eq!(
            f32,
            &[[0.25, 1.25], [0.0, 0.125]],
            &[[0.25, 1.25], [0.0, 0.125]],
        );
        assert_approx_eq!(
            f32,
            &[[0.25, 1.25], [0.0, 0.125]],
            arr2(&[[0.25, 1.25], [0.0, 0.125]]),
        );
        assert_approx_eq!(
            f32,
            &[[[0.25, 1.25], [0.0, 0.125]]],
            arr3(&[[[0.25, 1.25], [0.0, 0.125]]]),
        );
    }

    #[test]
    fn test_compares_nan_values() {
        assert_approx_eq!(f32, [3.1, f32::NAN, 1.0], [3.1, f32::NAN, 1.0]);
    }

    #[test]
    #[should_panic(expected = "[2]")]
    fn test_compares_nan_with_panic1() {
        assert_approx_eq!(f32, [3.1, f32::NAN, 1.0], [3.1, f32::NAN, 2.0]);
    }

    #[test]
    #[should_panic(expected = "[1]")]
    fn test_compares_nan_with_panic2() {
        assert_approx_eq!(f32, [3.1, f32::NAN, 1.0], [3.1, 3.0, 1.0]);
    }

    #[test]
    #[should_panic(expected = "[0, 2]")]
    fn test_panic_at_different_length() {
        assert_approx_eq!(f32, &[[1., 2., 3.]], &[[1., 2.]]);
    }

    #[test]
    fn test_equality_using_epsilon() {
        assert_approx_eq!(f32, 0.125, 0.625, epsilon = 0.5);
    }

    #[test]
    #[should_panic(expected = "[]")]
    fn test_equality_using_epsilon_with_panic() {
        assert_approx_eq!(f32, 0.125, 0.625, epsilon = 0.49);
    }
}
