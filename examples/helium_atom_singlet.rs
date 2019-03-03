#[macro_use]
extern crate ndarray;
use basis::Hydrogen1sBasis;
use metropolis::MetropolisBox;
use montecarlo::{Runner, Sampler};
use operator::{ElectronicHamiltonian, ElectronicPotential, IonicPotential, KineticEnergy};
use rand::{SeedableRng, StdRng};
use wavefunction::{JastrowSlater, Orbital};

fn main() {
    let optimal_width = 1.0 / 1.69;
    // setup basis set
    let ion_pos = array![[1.0, 0.0, 0.0]];
    let basis_set = Hydrogen1sBasis::new(ion_pos.clone(), vec![optimal_width]);

    // construct orbitals
    let orbitals = vec![
        Orbital::new(array![[1.0]], basis_set.clone()),
        Orbital::new(array![[1.0]], basis_set.clone()),
    ];

    // construct Jastrow-Slater wave function
    let wave_function = JastrowSlater::new(
        array![0.7, -0.01, -0.15], // Jastrow factor parameters
        orbitals,
        0.001, // scale distance
        1,     // number of electrons with spin up
    );

    // setup metropolis algorithm/Markov chain generator
    let metrop = MetropolisBox::from_rng(1.0, StdRng::from_seed([0; 32]));

    // Construct kinetic energy and ionic potential operators
    let kinetic = KineticEnergy::new();
    // One ion located at r = (0, 0, 0) with Z = 1
    let potential = IonicPotential::new(ion_pos, array![2]);
    // electron-electron interaction potential
    let potential_ee = ElectronicPotential::new();
    //  Full hamiltonian
    let hamiltonian =
        ElectronicHamiltonian::new(kinetic.clone(), potential.clone(), potential_ee.clone());

    // construct sampler
    let mut sampler = Sampler::new(wave_function, metrop);
    sampler.add_observable("Hamiltonian", hamiltonian);

    // create MC runner
    let mut runner = Runner::new(sampler);

    // Run Monte Carlo integration for 100000 steps, with block size 200
    runner.run(1_000_000, 250);

    //// Retrieve mean values of energy over run
    let energy = *runner.means().get("Hamiltonian").unwrap();
    let error_energy = *runner.errors().get("Hamiltonian").unwrap();

    println!(
        "\nEnergy:         {:.*} +/- {:.*}",
        8, energy, 8, error_energy
    );
    println!("Exact ground state energy: -2.903")
}