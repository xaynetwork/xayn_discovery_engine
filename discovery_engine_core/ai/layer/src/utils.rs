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

use std::f32::consts::SQRT_2;

use displaydoc::Display;
use ndarray::{
    Array2,
    ArrayBase,
    ArrayView1,
    Axis,
    DataMut,
    DataOwned,
    Dimension,
    IntoDimension,
    Ix2,
    IxDyn,
    NdFloat,
    RemoveAxis,
};
use rand::Rng;
use rand_distr::{num_traits::Float, Distribution, Normal};
use thiserror::Error;

/// Can't combine {name_left}({shape_left:?}) with {name_right}({shape_right:?}): {hint}
#[derive(Debug, Display, Error)]
#[allow(clippy::doc_markdown)] // false positive in combination with displaydoc
pub struct IncompatibleMatrices {
    name_left: &'static str,
    shape_left: IxDyn,
    name_right: &'static str,
    shape_right: IxDyn,
    hint: &'static str,
}

impl IncompatibleMatrices {
    pub fn new(
        name_left: &'static str,
        shape_left: impl IntoDimension,
        name_right: &'static str,
        shape_right: impl IntoDimension,
        hint: &'static str,
    ) -> Self {
        Self {
            name_left,
            shape_left: shape_left.into_dimension().into_dyn(),
            name_right,
            shape_right: shape_right.into_dimension().into_dyn(),
            hint,
        }
    }
}

/// Computes softmax along specified axis.
///
/// Inspired by [autograd's softmax implementation], especially the trick to subtract the max value
/// to reduce the chance of an overflow.
///
/// [autograd's softmax implementation]: https://docs.rs/autograd/1.0.0/src/autograd/ops/activation_ops.rs.html#59
pub fn softmax<A, S, D>(mut array: ArrayBase<S, D>, axis: Axis) -> ArrayBase<S, D>
where
    A: NdFloat,
    S: DataOwned<Elem = A> + DataMut<Elem = A>,
    D: Dimension + RemoveAxis,
{
    // Subtract `max` to prevent overflow, this
    // doesn't affect the outcome of the softmax.
    let max = array
        .fold_axis(axis, A::min_value(), |state, val| A::max(*state, *val))
        .insert_axis(axis);
    array -= &max;

    // Standard 3step softmax, 1) exp(x), 2) sum up, 3) divide through sum
    array.mapv_inplace(Float::exp);
    let sum = array.sum_axis(axis).insert_axis(axis);
    array /= &sum;
    array
}

/// Computes the Kullback-Leibler Divergence between a "good" distribution and one we want to evaluate.
///
/// Returns a result based on `nats`, i.e. it uses `ln` (instead of `log2` which
/// would produce a result based on `bits`).
///
/// All values are clamped/clipped to the range `f32::EPSILON..=1.`.
///
/// For the eval distribution this makes sense as we should never predict `0` but at most
/// a value so close to it, that it ends up as `0` due to the limited precision of
/// `f32`.
///
/// For the good distribution we could argue similarly. An alternative choice
/// is to return `0` if the good distributions probability is `0`.)
pub fn kl_divergence(good_dist: ArrayView1<f32>, eval_dist: ArrayView1<f32>) -> f32 {
    good_dist.into_iter().zip(eval_dist.into_iter()).fold(
        0.,
        |acc, (good_dist_prob, eval_dist_prob)| {
            let good_dist_prob = good_dist_prob.clamp(f32::EPSILON, 1.);
            let eval_dist_prob = eval_dist_prob.clamp(f32::EPSILON, 1.);
            acc + good_dist_prob * (good_dist_prob / eval_dist_prob).ln()
        },
    )
}

/// He-Uniform Initializer
///
/// Weights for layer `j` are sampled from following normal distribution:
///
/// ```ascii
/// W_j ~ N(μ=0, σ²=2/in_j)
/// ```
///
/// Where `n_j` is the number of input units of this layer.
/// This means for us `n_j` is the the number of rows of `W_j`.
///
/// Furthermore as we want to avoid exceedingly large values
/// we truncate the normal distribution at 2σ.
///
/// Source:
///
/// - [Website](https://www.cv-foundation.org/openaccess/content_iccv_2015/html/He_Delving_Deep_into_ICCV_2015_paper.html)
/// - [Pdf](https://www.cv-foundation.org/openaccess/content_iccv_2015/papers/He_Delving_Deep_into_ICCV_2015_paper.pdf)
#[allow(clippy::cast_precision_loss)] // our integers are small enough
#[allow(clippy::missing_panics_doc)] // edge cases are handled before unwrapping
pub fn he_normal_weights_init(
    rng: &mut (impl Rng + ?Sized),
    dim: impl IntoDimension<Dim = Ix2>,
) -> Array2<f32> {
    let dim = dim.into_dimension();
    let nr_rows = dim[0];

    // Avoids panic due to invalid σ which can only happen with empty weight matrices.
    if nr_rows == 0 {
        return Array2::default(dim);
    }

    let std_dev = SQRT_2 / (nr_rows as f32).sqrt();
    let dist = Normal::new(0., std_dev).unwrap();
    let limit = 2. * std_dev;

    Array2::from_shape_simple_fn(dim, || loop {
        let res = dist.sample(rng);
        if -limit <= res && res <= limit {
            break res;
        }
    })
}

#[cfg(test)]
mod tests {
    use ndarray::{arr1, arr2, arr3};

    use super::*;
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    #[test]
    fn test_softmax_1d() {
        let arr = arr1(&[-1., 0., 1., 2., 3.]);

        // axis 0
        let res = arr1(&[
            0.011_656_231,
            0.031_684_92,
            0.086_128_54,
            0.234_121_67,
            0.636_408_6,
        ]);
        assert_approx_eq!(f32, softmax(arr, Axis(0)), res);
    }

    #[test]
    fn test_softmax_2d() {
        let arr = arr2(&[
            [-1., 0., 1., 2., 3.],
            [9., 8., 7., 6., 5.],
            [1., -1., 1., -1., 1.],
        ])
        .into_shared();

        // axis 0
        let res = arr2(&[
            [
                0.000_045_382_647,
                0.000_335_308_78,
                0.002_466_524_7,
                0.017_970_119,
                0.117_310_42,
            ],
            [
                0.999_619_25,
                0.999_541_4,
                0.995_067,
                0.981_135_2,
                0.866_813_3,
            ],
            [
                0.000_335_334_92,
                0.000_123_353_21,
                0.002_466_524_7,
                0.000_894_679_5,
                0.015_876_24,
            ],
        ]);
        assert_approx_eq!(f32, softmax(arr.clone(), Axis(0)), res);

        // axis 1
        let res = arr2(&[
            [
                0.011_656_231,
                0.031_684_92,
                0.086_128_54,
                0.234_121_67,
                0.636_408_6,
            ],
            [
                0.636_408_6,
                0.234_121_67,
                0.086_128_54,
                0.031_684_92,
                0.011_656_231,
            ],
            [
                0.305_747_7,
                0.041_378_45,
                0.305_747_7,
                0.041_378_45,
                0.305_747_7,
            ],
        ]);
        assert_approx_eq!(f32, softmax(arr, Axis(1)), res);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_softmax_3d() {
        let arr = arr3(&[
            [
                [-1., 0., 1., 2., 3.],
                [9., 8., 7., 6., 5.],
                [1., -1., 1., -1., 1.],
            ],
            [
                [1., 1., 1., 1., 1.],
                [2., 2., 2., 2., 2.],
                [3., 3., 3., 3., 3.],
            ],
        ])
        .into_shared();

        // axis 0
        let res = arr3(&[
            [
                [0.119_202_92, 0.268_941_43, 0.5, 0.731_058_6, 0.880_797],
                [
                    0.999_089,
                    0.997_527_4,
                    0.993_307_2,
                    0.982_013_76,
                    0.952_574_13,
                ],
                [
                    0.119_202_92,
                    0.017_986_21,
                    0.119_202_92,
                    0.017_986_21,
                    0.119_202_92,
                ],
            ],
            [
                [0.880_797, 0.731_058_6, 0.5, 0.268_941_43, 0.119_202_92],
                [
                    0.000_911_051_23,
                    0.002_472_623_3,
                    0.006_692_851,
                    0.017_986_21,
                    0.047_425_874,
                ],
                [0.880_797, 0.982_013_76, 0.880_797, 0.982_013_76, 0.880_797],
            ],
        ]);
        assert_approx_eq!(f32, softmax(arr.clone(), Axis(0)), res);

        // axis 1
        let res = arr3(&[
            [
                [
                    0.000_045_382_647,
                    0.000_335_308_78,
                    0.002_466_524_7,
                    0.017_970_119,
                    0.117_310_42,
                ],
                [
                    0.999_619_25,
                    0.999_541_4,
                    0.995_067,
                    0.981_135_2,
                    0.866_813_3,
                ],
                [
                    0.000_335_334_92,
                    0.000_123_353_21,
                    0.002_466_524_7,
                    0.000_894_679_5,
                    0.015_876_24,
                ],
            ],
            [
                [
                    0.090_030_57,
                    0.090_030_57,
                    0.090_030_57,
                    0.090_030_57,
                    0.090_030_57,
                ],
                [
                    0.244_728_48,
                    0.244_728_48,
                    0.244_728_48,
                    0.244_728_48,
                    0.244_728_48,
                ],
                [
                    0.665_240_94,
                    0.665_240_94,
                    0.665_240_94,
                    0.665_240_94,
                    0.665_240_94,
                ],
            ],
        ]);
        assert_approx_eq!(f32, softmax(arr.clone(), Axis(1)), res);

        // axis 2
        let res = arr3(&[
            [
                [
                    0.011_656_232,
                    0.031_684_92,
                    0.086_128_54,
                    0.234_121_67,
                    0.636_408_6,
                ],
                [
                    0.636_408_6,
                    0.234_121_67,
                    0.086_128_54,
                    0.031_684_92,
                    0.011_656_231,
                ],
                [
                    0.305_747_7,
                    0.041_378_45,
                    0.305_747_7,
                    0.041_378_45,
                    0.305_747_7,
                ],
            ],
            [
                [0.2, 0.2, 0.2, 0.2, 0.2],
                [0.2, 0.2, 0.2, 0.2, 0.2],
                [0.2, 0.2, 0.2, 0.2, 0.2],
            ],
        ]);
        assert_approx_eq!(f32, softmax(arr, Axis(2)), res);
    }

    #[test]
    fn test_softmax_edgecases() {
        // 2D axis 0
        let arr = arr2(&[[-1., 0., 1., 2., 3.]]);
        let res = arr2(&[[1., 1., 1., 1., 1.]]);
        assert_approx_eq!(f32, softmax(arr, Axis(0)), res);

        // 2D axis 1
        let arr = arr2(&[[-1.], [9.], [1.]]);
        let res = arr2(&[[1.], [1.], [1.]]);
        assert_approx_eq!(f32, softmax(arr, Axis(1)), res);

        // 3D axis 0
        let arr = arr3(&[[
            [-1., 0., 1., 2., 3.],
            [9., 8., 7., 6., 5.],
            [1., -1., 1., -1., 1.],
        ]]);
        let res = arr3(&[[
            [1., 1., 1., 1., 1.],
            [1., 1., 1., 1., 1.],
            [1., 1., 1., 1., 1.],
        ]]);
        assert_approx_eq!(f32, softmax(arr, Axis(0)), res);

        // 3D axis 1
        let arr = arr3(&[[[-1., 0., 1., 2., 3.]], [[1., 1., 1., 1., 1.]]]);
        let res = arr3(&[[[1., 1., 1., 1., 1.]], [[1., 1., 1., 1., 1.]]]);
        assert_approx_eq!(f32, softmax(arr, Axis(1)), res);

        // 3D axis 2
        let arr = arr3(&[[[-1.], [9.], [1.]], [[1.], [2.], [3.]]]);
        let res = arr3(&[[[1.], [1.], [1.]], [[1.], [1.], [1.]]]);
        assert_approx_eq!(f32, softmax(arr, Axis(2)), res);
    }

    #[test]
    fn test_kl_divergence_calculation() {
        let good_dist = arr1(&[0.5, 0.1, 0.025, 0.3, 0.075]);
        let eval_dist = arr1(&[0.3, 0.2, 0.15, 0.2, 0.15]);

        let cost = kl_divergence(good_dist.view(), eval_dist.view());

        assert_approx_eq!(f32, cost, 0.210_957_6);
    }

    #[test]
    fn test_kl_divergence_calculation_handles_zeros() {
        let good_dist = arr1(&[0.0, 0.1, 0.0, 0.3, 0.075]);
        let eval_dist = arr1(&[0.0, 0.2, 0.15, 0.0, 0.15]);

        let cost = kl_divergence(good_dist.view(), eval_dist.view());

        assert_approx_eq!(f32, cost, 4.300_221_4);
    }

    #[test]
    fn test_he_normal_weight_init_zero_dimensions() {
        let mut rng = rand::thread_rng();
        assert_eq!(
            he_normal_weights_init(&mut rng, (0, 200)).shape(),
            &[0, 200],
        );
        assert_eq!(
            he_normal_weights_init(&mut rng, (300, 0)).shape(),
            &[300, 0],
        );
        assert_eq!(he_normal_weights_init(&mut rng, (0, 0)).shape(), &[0, 0]);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // our integers are small enough
    fn test_he_normal_weight_init() {
        let mut rng = rand::thread_rng();
        let weights = he_normal_weights_init(&mut rng, (300, 200));

        assert_eq!(weights.shape(), &[300, 200]);

        let std = SQRT_2 / (weights.shape()[0] as f32).sqrt();
        let limit = 2. * std;
        let mut c_1std = 0;
        let mut c_2std = 0;
        for &w in weights.iter() {
            assert!(
                -limit <= w && w <= limit,
                "out of bound weight: {} <= {} <= {}",
                -limit,
                w,
                limit
            );
            if -std <= w && w <= std {
                c_1std += 1;
            } else {
                c_2std += 1;
            }
        }

        let nr_weights = weights.len() as f32;
        let prob_1std = c_1std as f32 / nr_weights;
        let prob_2std = c_2std as f32 / nr_weights;

        // Probabilities of a weight being in +-1std or +-2std corrected
        // wrt. the truncating of values outside of +-2std. We accept this
        // to be true if the found percentage is around +-5% of the expected
        // percentage.
        assert_approx_eq!(f32, prob_1std, 0.715_232_8, epsilon = 0.05);
        assert_approx_eq!(f32, prob_2std, 0.284_767_2, epsilon = 0.05);
    }
}
