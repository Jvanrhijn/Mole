use std::collections::HashMap;
#[macro_use]
extern crate ndarray;
use basis::GaussianBasis;
use metropolis::MetropolisBox;
use montecarlo::{traits::Log, Runner, Sampler};
use ndarray::{Array1, Array2, Axis, Ix2};
use ndarray_linalg::Norm;
use operator::{KineticEnergy, Operator, OperatorValue};
use rand::{SeedableRng, StdRng};
use wavefunction::{Cache, Differentiate, Error, Orbital, SingleDeterminant};

// Create a very basic logger
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
        )
    }
}

// Create a struct to hold Hamiltonian parameters
struct HarmonicHamiltonian {
    // Harmonic oscillator potential is parametrized by natural frequency
    frequency: f64,
    // Kinetic energy operator always has the same form
    t: KineticEnergy,
}

impl HarmonicHamiltonian {
    pub fn new(frequency: f64) -> Self {
        Self {
            t: KineticEnergy::new(),
            frequency,
        }
    }
}

// All observables must implement the Operator<T> trait
// T is the type parameter of the wave function.
impl<T> Operator<T> for HarmonicHamiltonian
where
    T: Differentiate<D = Ix2> + Cache,
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<OperatorValue, Error> {
        // Kinetic energy
        let ke = self.t.act_on(wf, cfg)?;
        // Potential energy: V = 0.5*m*omega^2*|x|^2
        let pe = OperatorValue::Scalar(
            0.5 * self.frequency.powi(2) * cfg.norm_l2().powi(2) * wf.current_value().0,
        );
        Ok(&ke + &pe)
    }
}

fn main() {
    // Exact ground state of Harmonic oscillator
    let basis = GaussianBasis::new(array![[0.0, 0.0, 0.0]], vec![1.0]);
    // the rest is the same as with other operators

    // Build wave function
    let orbital = Orbital::new(array![[1.0]], basis);
    let ansatz = SingleDeterminant::new(vec![orbital]);

    let metrop = MetropolisBox::from_rng(1.0, StdRng::from_seed([0; 32]));

    // Construct our custom operator
    let hamiltonian = HarmonicHamiltonian::new(1.0);

    let mut sampler = Sampler::new(ansatz, metrop);
    sampler.add_observable("Energy", hamiltonian);

    // Perform the MC integration
    let mut runner = Runner::new(sampler, Logger);
    runner.run(1000, 1);

    let energy_data = Array1::<f64>::from_vec(
        runner
            .data()
            .get("Energy")
            .unwrap()
            .iter()
            .map(|x| *x.get_scalar().unwrap())
            .collect::<Vec<_>>(),
    );

    // Retrieve mean values of energy over run
    let energy = *energy_data.mean_axis(Axis(0)).first().unwrap();
    let error = *energy_data.std_axis(Axis(0), 0.0).first().unwrap();

    assert_eq!(energy, 1.5);
    assert!(error < 1e-15);

    println!("\nEnergy:     {} +/- {:.*}", energy, 8, error);
}
