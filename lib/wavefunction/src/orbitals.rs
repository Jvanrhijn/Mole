// Standard imports
use std::vec::Vec;
// Third party imports
use ndarray::{Array, Array1, Ix1};
// First party imports
use crate::traits::{Function, Differentiate};
use crate::error::Error;
use basis::Vgl;

/// Parametrized orbital as a linear combination of basis functions:
/// $\phi(x) = \sum_{i=1}^{N_{\text{basis}}} \xi_i(x)$.
pub struct Orbital<'a, T: 'a>
    where T: ?Sized + Fn(&Array1<f64>) -> Vgl
{
    parms: Array1<f64>,
    basis_set: &'a Vec<Box<T>>
}

impl<'a, T> Orbital<'a, T>
    where T: ?Sized + Fn(&Array1<f64>) -> Vgl
{
    pub fn new(parms: Array1<f64>, basis_set: &'a Vec<Box<T>>) -> Self {
        Self{parms, basis_set}
    }
}

impl<'a, T> Function<f64> for Orbital<'a, T>
    where T: ?Sized + Fn(&Array1<f64>) -> Vgl {

    type D = Ix1;

    fn value(&self, cfg: &Array<f64, Self::D>) -> Result<f64, Error> {
        let basis_vals = self.basis_set.iter().map(|x| x(cfg).0).collect();
        Ok((Array1::from_vec(basis_vals) * &self.parms).scalar_sum())
    }
}

impl<'a, T> Differentiate for Orbital<'a, T>
    where T: ?Sized + Fn(&Array1<f64>) -> Vgl {

    type D = Ix1;

    fn gradient(&self, _cfg: &Array<f64, Self::D>) -> Result<Array<f64, Self::D>, Error> {
        unimplemented!()
    }

    fn laplacian(&self, cfg: &Array<f64, Self::D>) -> Result<f64, Error> {
        Ok(self.parms.iter().zip(self.basis_set).map(|(x, y)| x*y(cfg).2).sum())
    }

}
