#[macro_use]
extern crate ndarray;
use ndarray::arr2;
use rand::{SeedableRng, StdRng};

use basis::GaussianBasis;
use metropolis::MetropolisBox;
use montecarlo::{Runner, Sampler};

use operator::{ElectronicHamiltonian, ElectronicPotential, IonicPotential, KineticEnergy};
use wavefunction::{JastrowFactor, JastrowSlater, Orbital, SingleDeterminant};

fn main() {
    let ion_positions = array![[-0.7, 0.0, 0.0], [0.7, 0.0, 0.0]];

    let basis_set = GaussianBasis::new(ion_positions.clone(), vec![2.5]);

    let orbital1 = Orbital::new(array![[1.0], [1.0]], basis_set.clone());
    let orbital2 = Orbital::new(array![[1.0], [1.0]], basis_set);

    let det_up = SingleDeterminant::new(vec![orbital1]);
    let det_down = SingleDeterminant::new(vec![orbital2]);
    let jas = JastrowFactor::new(array![1.0, 0.25], 2, 0.1);
    let wave_func = JastrowSlater::new(det_up, det_down, jas);

    let kinetic = KineticEnergy::new();
    let potential_ions = IonicPotential::new(ion_positions, array![1, 1]);
    let potential_electrons = ElectronicPotential::new();
    let hamiltonian = ElectronicHamiltonian::new(
        kinetic.clone(),
        potential_ions.clone(),
        potential_electrons.clone(),
    );

    let metrop = MetropolisBox::from_rng(1.5, StdRng::from_seed([0; 32]));

    let mut sampler = Sampler::new(wave_func, metrop);
    sampler.add_observable("Kinetic Energy", kinetic);
    sampler.add_observable("Ion Potential", potential_ions);
    sampler.add_observable("Electron Potential", potential_electrons);
    sampler.add_observable("Energy", hamiltonian);

    let mut runner = Runner::new(sampler);
    runner.run(100_000, 200);

    let total_energy = *runner.means().get("Energy").unwrap();
    let energy_variance = *runner.variances().get("Energy").unwrap();

    println!(
        "\nTotal Energy: {e:.*} +/- {s:.*}",
        8,
        8,
        e = total_energy,
        s = energy_variance.sqrt()
    );
}
