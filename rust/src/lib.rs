pub mod rtdata;

use pyo3::prelude::*;
use rtdata::server::serve;

#[pymodule]
fn rustmod(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(serve, m)?)?;
    Ok(())
}
