// Third party imports
use ndarray::{Array1};

/// Return value and laplacian of the 1s hydrogen orbital
pub fn hydrogen_1s(pos: &Array1<f64>) -> (f64, f64) {
    let r = (pos*pos).scalar_sum().sqrt();
    let exp = (-r).exp();
    (exp, (1. - 2./r)*(exp))
}

/// Return value and laplacian of the 2s hydrogen orbital
pub fn hydrogen_2s(pos: &Array1<f64>) -> (f64, f64) {
    let r = (pos*pos).scalar_sum().sqrt();
    let exp = (-r/2.).exp();
    ((1. - r/2.)*exp, exp/8.*(10. - r - 16./r))
}

/// Return value and Laplacian of Gaussian orbital
pub fn gaussian(pos: &Array1<f64>, width: f64) -> (f64, f64) {
    let r = (pos*pos).scalar_sum().sqrt();
    let exp = (-(r).powi(2)/(2.*width).powi(2)).exp();
    (exp, exp/8.*(r.powi(2) - 6.*width.powi(2))/(4.*width.powi(4)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Range;
    use ndarray_rand::RandomExt;

    #[test]
    fn ground_state() {
        let r = Array1::<f64>::random(3, Range::new(-1., 1.));
        let wf_val = hydrogen_1s(&r).0;
        assert!(wf_val > 0.);
    }
}
