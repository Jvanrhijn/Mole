use mole::prelude::*;
use ndarray::{Array1, Array2, Array, Ix1, Ix2, Axis, array, s};
use ndarray_linalg::Norm;
use std::collections::HashMap;
use rand::{SeedableRng, StdRng};

use rand::distributions::{Normal, Uniform, WeightedChoice, Weighted, Distribution};
use rand::{FromEntropy, Rng, RngCore};
use ndarray_rand::RandomExt;
use gnuplot::{AxesCommon, Caption, Color, Figure, FillAlpha};
use itertools::izip;

// DMC test for hydrogen atom,
// testing ground for library integration of
// DMC algorithm

// Create a very basic logger
#[derive(Clone)]
struct Logger;
impl Log for Logger {
    fn log(&mut self, data: &HashMap<String, Vec<OperatorValue>>) -> String {
        format!(
            "Energy: {}",
            data.get("Energy")
                .unwrap()
                .iter()
                .last()
                .unwrap()
                .get_scalar()
                .unwrap()
        );
        String::new()
    }
}

// hydrogen atom trial function gaussian#[derive(Clone)]
#[derive(Clone)]
struct GaussianWaveFunction {
    params: Array1<f64>,
}

impl GaussianWaveFunction {
    pub fn new(a: f64) -> Self {
        Self { params: array![a] }
    }
}

impl Function<f64> for GaussianWaveFunction {
    type D = Ix2;

    fn value(&self, x: &Array2<f64>) -> Result<f64> {
        let a = self.params[0];
        Ok(f64::exp(-(x.norm_l2()/a).powi(2)))
    }
}

impl Differentiate for GaussianWaveFunction {
    type D = Ix2;

    fn gradient(&self, x: &Array2<f64>) -> Result<Array2<f64>> {
        let a = self.params[0];
        Ok(-2.0*self.value(x)?/a.powi(2)*x)
    }

    fn laplacian(&self, x: &Array2<f64>) -> Result<f64> {
        let a = self.params[0];
        Ok(
            self.value(x)?*(4.0*x.norm_l2().powi(2) - 6.0*a.powi(2))/a.powi(4)
        )
    }
}

impl WaveFunction for GaussianWaveFunction {
    fn num_electrons(&self) -> usize {
        1
    }
}

impl Optimize for GaussianWaveFunction {
    fn parameter_gradient(&self, cfg: &Array2<f64>) -> Result<Array1<f64>> {
        let a = self.params[0];
        Ok(array![self.value(cfg)?*2.0*cfg.norm_l2().powi(2)/a.powi(3)])
    }

    fn update_parameters(&mut self, deltap: &Array1<f64>) {
        self.params += deltap;
    }

    fn parameters(&self) -> &Array1<f64> {
        &self.params
    }

    fn num_parameters(&self) -> usize {
        self.params.len()
    }
}

static ITERS: usize = 100;
static TOTAL_SAMPLES: usize = 5000;
static BLOCK_SIZE: usize = 10;

fn main() {
    // Build wave function
    let ansatz = GaussianWaveFunction::new(1.0);

    let metrop = MetropolisDiffuse::from_rng(0.1, StdRng::from_seed([0; 32]));

    // Construct our custom operator
    let hamiltonian = ElectronicHamiltonian::from_ions(array![[0.0, 0.0, 0.0]], array![1]);

    let obs = operators! {
        "Energy" => hamiltonian.clone(),
        "Parameter gradient" => ParameterGradient,
        "Wavefunction value" => WavefunctionValue
    };

    let sampler = Sampler::new(ansatz, metrop, &obs).expect("Bad initial configuration");

    // first do a VMC run to obtain a variationally optimized wave function
    let vmc = VmcRunner::new(sampler, SteepestDescent::new(1e-5), Logger);

    let (guiding_wf, energies, errors) = vmc.run_optimization(ITERS, TOTAL_SAMPLES, BLOCK_SIZE, 4)
                                            .expect("VMC run failed");

    let vmc_energy = energies.iter().last().unwrap();
    let error = errors.iter().last().unwrap();

    println!("\nVMC Energy:     {} +/- {:.*}\n", vmc_energy, 8, error);

    // Improve the energy by DMC

    // sample a set of starting configurations
    // from the wave function
    let num_confs = 1000;
    const TAU: f64 = 1e-2;
    const DMC_ITERS: usize = 1000;
    const EQ_ITERS: usize = DMC_ITERS / 10;

    // initialize trial energy
    let trial_energy = *vmc_energy;

    let rng = StdRng::from_seed([1_u8; 32]);
    let mut dmc = DmcRunner::with_rng(guiding_wf, num_confs, trial_energy, hamiltonian, rng);

    let (dmc_energy, dmc_vars) = dmc.diffuse(TAU, DMC_ITERS, EQ_ITERS);
  

    println!("\nDMC Energy:   {:.8} +/- {:.8}", dmc_energy.last().unwrap(), dmc_vars.last().unwrap().sqrt()); 

    plot_results(&dmc_energy.into(), &dmc_vars.iter().map(|x| x.sqrt()/((DMC_ITERS - EQ_ITERS) as f64).sqrt()).collect::<Array1<f64>>(), "blue");
}

fn plot_results(
    energies: &Array1<f64>,
    errors: &Array1<f64>,
    color: &str,
) {
    let niters = energies.len();
    let iters: Vec<_> = (0..niters).collect();
    let exact = vec![-0.5; niters];

    let mut fig = Figure::new();
    let axes = fig.axes2d();
    axes.fill_between(
        &iters,
        &(energies - errors),
        &(energies + errors),
        &[Color(color), FillAlpha(0.1)],
    )
    .lines(&iters, energies, &[Caption("DMC energy"), Color(color)]);
    axes.lines(
        &iters,
        &exact,
        &[Caption("Exact ground state energy, Hydrogen"), Color("black")],
    )
    .set_x_label("Iteration", &[])
    .set_y_label("Exact energy (Hartree)", &[])
    .set_x_grid(true)
    .set_y_grid(true);

    fig.show();
}