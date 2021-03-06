use ndarray::{Array, Array1, Array2, Ix2};
use ndarray_linalg::Norm;
use ndarray_rand::RandomExt;
use rand::distributions::{Normal, Range};
use rand::rngs::StdRng;
use rand::{FromEntropy, Rng, SeedableRng};

use crate::traits::Metropolis;
use errors::Error;
use wavefunction_traits::{Differentiate, Function};

type Result<T> = std::result::Result<T, Error>;

#[allow(dead_code)]
type Vgl = (f64, Array2<f64>, f64);

/// Simplest Metropolis algorithm.
/// Transition matrix T(x -> x') is constant inside a cubical box,
/// and zero outside it. This yields an acceptance probability of
/// $A(x -> x') = \min(\psi(x')^2 / \psi(x)^2, 1)$.
#[derive(Clone)]
pub struct MetropolisBox<R>
where
    R: Rng,
{
    box_side: f64,
    rng: R,
}

impl<R> MetropolisBox<R>
where
    R: Rng,
{
    pub fn from_rng(box_side: f64, rng: R) -> Self {
        Self { box_side, rng }
    }
}

impl MetropolisBox<StdRng> {
    pub fn new(box_side: f64) -> Self {
        Self {
            box_side,
            rng: StdRng::from_entropy(),
        }
    }
}

impl<T, R> Metropolis<T> for MetropolisBox<R>
where
    T: Function<f64, D = Ix2> + Clone,
    R: Rng + SeedableRng,
    <R as SeedableRng>::Seed: From<[u8; 32]>,
{
    type R = R;

    fn rng_mut(&mut self) -> &mut R {
        &mut self.rng
    }

    fn propose_move(&mut self, wf: &mut T, cfg: &Array2<f64>, idx: usize) -> Result<Array2<f64>> {
        let mut config_proposed = cfg.clone();
        {
            let mut mov_slice = config_proposed.slice_mut(s![idx, ..]);
            mov_slice += &Array1::random_using(
                3,
                Range::new(-0.5 * self.box_side, 0.5 * self.box_side),
                &mut self.rng,
            );
        }
        Ok(config_proposed)
    }

    fn accept_move(
        &mut self,
        wf: &mut T,
        cfg: &Array2<f64>,
        cfg_prop: &Array2<f64>,
    ) -> Result<bool> {
        let wf_value = wf.value(cfg_prop)?;
        let acceptance = (wf_value.powi(2) / wf.value(cfg)?.powi(2)).min(1.0);
        Ok(acceptance > self.rng.gen::<f64>())
    }

    fn move_state(
        &mut self,
        wf: &mut T,
        cfg: &Array2<f64>,
        idx: usize,
    ) -> Result<Option<Array2<f64>>> {
        let cfg_proposed = self.propose_move(wf, cfg, idx)?;
        if self.accept_move(wf, cfg, &cfg_proposed)? {
            Ok(Some(cfg_proposed))
        } else {
            Ok(None)
        }
    }

    fn reseed_rng(&mut self, s: [u8; 32]) {
        self.rng = Self::R::from_seed(s.into());
    }
}

#[derive(Clone)]
pub struct MetropolisDiffuse<R>
where
    R: Rng,
{
    time_step: f64,
    fixed_node: bool,
    rng: R,
}

impl<R: Rng> MetropolisDiffuse<R> {
    pub fn from_rng(time_step: f64, rng: R) -> Self {
        Self {
            time_step,
            fixed_node: false,
            rng,
        }
    }

    pub fn fix_nodes(mut self) -> Self {
        self.fixed_node = true;
        self
    }
}

impl MetropolisDiffuse<StdRng> {
    pub fn new(time_step: f64) -> Self {
        Self {
            time_step,
            fixed_node: false,
            rng: StdRng::from_entropy(),
        }
    }
}

impl<T, R> Metropolis<T> for MetropolisDiffuse<R>
where
    T: Differentiate<D = Ix2> + Function<f64, D = Ix2> + Clone,
    R: Rng + SeedableRng,
    <R as SeedableRng>::Seed: From<[u8; 32]>,
{
    type R = R;

    fn rng_mut(&mut self) -> &mut R {
        &mut self.rng
    }

    fn propose_move(&mut self, wf: &mut T, cfg: &Array2<f64>, idx: usize) -> Result<Array2<f64>> {
        let mut config_proposed = cfg.clone();
        {
            let wf_value = wf.value(cfg)?;
            let wf_grad = wf.gradient(cfg)?;
            let drift_velocity = &wf_grad.slice(s![idx, ..]) / wf_value;

            let mut mov_slice = config_proposed.slice_mut(s![idx, ..]);
            mov_slice += &(drift_velocity * self.time_step);
            mov_slice +=
                &Array1::random_using(3, Normal::new(0.0, self.time_step.sqrt()), &mut self.rng);
        }
        Ok(config_proposed)
    }

    fn accept_move(
        &mut self,
        wf: &mut T,
        cfg: &Array2<f64>,
        cfg_prop: &Array2<f64>,
    ) -> Result<bool> {
        let wf_value = wf.value(cfg_prop)?;
        let wf_grad = wf.gradient(cfg_prop)?;
        let drift_velocity = &wf_grad / wf_value;
        let wf_value_old = wf.value(cfg)?;
        let wf_grad_old = wf.gradient(cfg)?;
        let drift_velocity_old = &wf_grad_old / wf_value_old;

        if wf_value.signum() != wf_value_old.signum() {
            return Ok(false);
        }

        let t_high = f64::exp(
            -(&(cfg - cfg_prop) - &(&drift_velocity * self.time_step))
                .norm_l2()
                .powi(2)
                / (2.0 * self.time_step),
        );
        let t_low = f64::exp(
            -(&(cfg_prop - cfg) - &(&drift_velocity_old * self.time_step))
                .norm_l2()
                .powi(2)
                / (2.0 * self.time_step),
        );

        let acceptance = (t_high * wf_value.powi(2) / (t_low * wf_value_old.powi(2))).min(1.0);

        Ok(acceptance > self.rng.gen::<f64>())
    }

    fn move_state(
        &mut self,
        wf: &mut T,
        cfg: &Array2<f64>,
        idx: usize,
    ) -> Result<Option<Array2<f64>>> {
        let cfg_proposed = self.propose_move(wf, cfg, idx)?;
        if self.accept_move(wf, cfg, &cfg_proposed)? {
            Ok(Some(cfg_proposed))
        } else {
            Ok(None)
        }
    }

    fn reseed_rng(&mut self, s: [u8; 32]) {
        self.rng = Self::R::from_seed(s.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::vec, num, prelude::*};

    // define stub wave function
    #[derive(Clone)]
    struct WaveFunctionMock {
        value: f64,
    }

    #[allow(dead_code)]
    impl WaveFunctionMock {
        pub fn set_value(&mut self, new_val: f64) {
            self.value = new_val;
        }
    }

    impl Function<f64> for WaveFunctionMock {
        type D = Ix2;

        fn value(&self, _cfg: &Array2<f64>) -> Result<f64> {
            Ok(self.value)
        }
    }

    impl Differentiate for WaveFunctionMock {
        type D = Ix2;

        fn gradient(&self, _cfg: &Array2<f64>) -> Result<Array2<f64>> {
            unimplemented!()
        }

        fn laplacian(&self, _cfg: &Array2<f64>) -> Result<f64> {
            Ok(1.0)
        }
    }

    type Ovgl = (Option<f64>, Option<Array2<f64>>, Option<f64>);

    proptest! {
        #[test]
        fn test_uniform_wf(v in vec(num::f64::NORMAL, 3)) {
            //let cfg = Array2::<f64>::ones((1, 3));
            let cfg = Array2::<f64>::from_shape_vec((1, 3), v).unwrap();
            let mut wf = WaveFunctionMock { value: 1.0 };
            let mut metrop = MetropolisBox::<StdRng>::new(1.0);
            let new_cfg = metrop.propose_move(&mut wf, &cfg, 0).unwrap(); // should always accept
            assert!(metrop.accept_move(&mut wf, &cfg, &new_cfg).unwrap());
        }
    }
}
