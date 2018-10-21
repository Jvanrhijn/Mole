// Third party imports
use ndarray::{Array, Array1, Array2, Ix2, Axis};
// First party imports
use traits::operator::Operator;
use traits::function::Function;
use traits::differentiate::Differentiate;
use error::{Error};

/// Ionic potential energy operator:
/// $\hat{V}_{\mathrm{ion}} = -\sum_{i=1}^{N_{\mathrm{ions}}\sum_{j=1}^{\mathrm{e}} \frac{Z_i}{r_{ij}}$.
/// Ionic charges are in units of the proton charge.
pub struct IonicPotential {
    ion_positions: Array2<f64>,
    ion_charge: Array1<i32>
}

impl Function<f64> for IonicPotential {
    type D = Ix2;

    fn value(&self, cfg: &Array2<f64>) -> Result<f64, Error> {
        let num_ions = self.ion_positions.len_of(Axis(0));
        let num_elec = cfg.len_of(Axis(0));
        let mut pot = 0.;
        for i in 0..num_ions {
            for j in 0..num_elec {
                let separation = &cfg.slice(s![j, ..]) - &self.ion_positions.slice(s![i, ..]);
                let distance = separation.dot(&separation).sqrt();
                pot -= self.ion_charge[i] as f64/distance;
            }
        }
        Ok(pot)
    }

}

impl IonicPotential {
    pub fn new(ion_positions: Array2<f64>, ion_charge: Array1<i32>) -> Self {
        IonicPotential{ion_positions, ion_charge}
    }
}

impl<T> Operator<T> for IonicPotential
where T: Function<f64, D=Ix2> + ?Sized,
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<f64, Error> {
        Ok(self.value(cfg)?*wf.value(cfg)?)
    }
}

/// Electron-electron interaction potential:
/// $\hat{V}_{ee} = \sum_{i=1}^{N_e} \sum_{j>i}^{N_e} \frac{1}{r_{ij}}.$
/// This potential is only a function of the current electronic configuration, and
/// so is not parametrized over anything.
pub struct ElectronicPotential {}

impl ElectronicPotential {
    pub fn new() -> Self {
        ElectronicPotential{}
    }
}

impl Function<f64> for ElectronicPotential {

    type D = Ix2;

    fn value(&self, cfg: &Array2<f64>) -> Result<f64, Error> {
        let num_elec = cfg.len_of(Axis(0));
        let mut pot = 0.;
        for i in 0..num_elec {
            for j in i+1..num_elec {
                let separation = &cfg.slice(s![i, ..]) - &cfg.slice(s![j, ..]);
                pot += 1./separation.dot(&separation).sqrt();
            }
        }
        Ok(pot)
    }
}

impl<T> Operator<T> for ElectronicPotential
where T: Function<f64, D=Ix2> + ?Sized,
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<f64, Error> {
        Ok(self.value(cfg)?*wf.value(cfg)?)
    }
}

/// Kinetic energy operator:
/// $\hat{T} = -\frac{1}{2}\sum_{i=1}^{N_e}\nabla_{i}^2$.
/// The kinetic energy is solely a function of the current electronic configuration,
/// and so it is not parametrized over anything.
pub struct KineticEnergy {}

impl KineticEnergy {
    pub fn  new() -> Self {
        KineticEnergy{}
    }
}

impl<T> Operator<T> for KineticEnergy
where T: Function<f64, D=Ix2> + Differentiate<D=Ix2> + ?Sized,
{
    fn act_on(&self, wf: &T, cfg: &Array<f64, Ix2>) -> Result<f64, Error> {
        Ok(-0.5*wf.laplacian(cfg)?)
    }
}

/// Ionic Hamiltonian operator:
/// $\hat{H} = \hat{T} + \hat{V}_{\mathrm{ion}}$.
/// Use this for calculations neglecting electron-electron interactions.
pub struct IonicHamiltonian {
    v: IonicPotential,
    t: KineticEnergy
}

impl IonicHamiltonian {
    pub fn new(t: KineticEnergy, v: IonicPotential) -> Self {
        IonicHamiltonian{v, t}
    }
}

impl<T> Operator<T> for IonicHamiltonian
where T: Function<f64, D=Ix2> + Differentiate<D=Ix2> + ?Sized,
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<f64, Error> {
        Ok(self.t.act_on(wf, cfg)? + self.v.act_on(wf, cfg)?)
    }
}

/// Electronic Hamiltonian operator:
/// $\hat{H} = \hat{T} + \hat{V}_{\mathrm{ion}} + \hat{V}_{\mathrm{ee}}$.
/// Use this operator for constructing a proper local energy operator. The hamiltonian
/// simply acts on any type implementing the WaveFunction and Function traits,
/// but does not care about normalization.
pub struct ElectronicHamiltonian {
    t: KineticEnergy,
    vion: IonicPotential,
    velec: ElectronicPotential
}

impl ElectronicHamiltonian {
    pub fn new(t: KineticEnergy, vion: IonicPotential, velec: ElectronicPotential) -> Self {
        ElectronicHamiltonian{t, vion, velec}
    }
}

impl<T> Operator<T> for ElectronicHamiltonian
where T: Function<f64, D=Ix2> + Differentiate<D=Ix2> + ?Sized
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<f64, Error> {
        Ok(self.t.act_on(wf, cfg)? + self.vion.act_on(wf, cfg)? + self.velec.act_on(wf, cfg)?)
    }
}

/// Local energy operator:
/// $\hat{E}_{L}\psi(x) = \frac{\hat{H}\psi}{\psi(x)}$.
/// This operator should be used in any Monte Carlo simulation
/// trying to calculate the electronic ground state of a molecular system.
pub struct LocalEnergy {
    h: ElectronicHamiltonian
}

impl LocalEnergy {
    pub fn new(h: ElectronicHamiltonian) -> Self {
        LocalEnergy{h}
    }
}

impl<T> Operator<T> for LocalEnergy
where T: Function<f64, D=Ix2> + Differentiate<D=Ix2> + ?Sized
{
    fn act_on(&self, wf: &T, cfg: &Array2<f64>) -> Result<f64, Error> {
        Ok(self.h.act_on(wf, cfg)?/wf.value(cfg)?)
    }
}
