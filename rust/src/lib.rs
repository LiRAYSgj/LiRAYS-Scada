pub mod rtdata;

use pyo3::prelude::*;
use rtdata::server::serve;
use rtdata::utils::generate_json_examples;

#[pymodule]
fn rustmod(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(serve, m)?)?;
    m.add_function(wrap_pyfunction!(generate_json_examples, m)?)?;
    Ok(())
}
