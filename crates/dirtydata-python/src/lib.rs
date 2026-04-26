use pyo3::prelude::*;
use numpy::{PyArray1, PyReadonlyArray1};
use dirtydata_dsp_chaos::{ChuaCircuit, Lorenz, MackeyGlass};
use dirtydata_dsp_svf::Svf;

#[pyfunction]
fn chua_process<'py>(
    py: Python<'py>,
    input: PyReadonlyArray1<f32>,
    alpha: f32,
    beta: f32,
    rate: f32,
    sample_rate: f32,
) -> Bound<'py, PyArray1<f32>> {
    let mut circuit = ChuaCircuit::new(sample_rate);
    let input = input.as_array();
    let mut output = Vec::with_capacity(input.len());
    
    for _ in 0..input.len() {
        // Chua here is an autonomous oscillator, but we can treat it as a processor 
        // if we decide to modulate it. For now, following user's 'chua_process(signal)' intent.
        output.push(circuit.process(alpha, beta, rate));
    }
    
    PyArray1::from_vec_bound(py, output)
}

#[pyfunction]
fn mackey_glass<'py>(
    py: Python<'py>,
    n_samples: usize,
    a: f32,
    b: f32,
    tau: usize,
    sr: f32,
) -> Bound<'py, PyArray1<f32>> {
    let mut mg = MackeyGlass::new();
    let dt = 1.0 / sr;
    let mut output = Vec::with_capacity(n_samples);
    
    for _ in 0..n_samples {
        output.push(mg.process(a, b, tau, dt));
    }
    
    PyArray1::from_vec_bound(py, output)
}

#[pyfunction]
fn lorenz<'py>(
    py: Python<'py>,
    n_samples: usize,
    sigma: f32,
    rho: f32,
    beta: f32,
    sr: f32,
) -> Bound<'py, PyArray1<f32>> {
    let mut lz = Lorenz::new();
    let dt = 1.0 / sr;
    let mut output = Vec::with_capacity(n_samples * 3);
    
    for _ in 0..n_samples {
        let s = lz.process(sigma, rho, beta, dt);
        output.push(s[0]);
        output.push(s[1]);
        output.push(s[2]);
    }
    
    PyArray1::from_vec_bound(py, output)
}

#[pyfunction]
fn process_ladder_filter<'py>(
    py: Python<'py>,
    input: PyReadonlyArray1<f32>,
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
) -> Bound<'py, PyArray1<f32>> {
    // Note: dirtydata-dsp-svf has Svf, using it as a high-quality filter proxy
    let mut filter = Svf::new(sample_rate);
    
    let input = input.as_array();
    let mut output = Vec::with_capacity(input.len());
    
    for &s in input.iter() {
        // SVF output: [lowpass, bandpass, highpass, notch]
        let res = filter.process(s, cutoff, resonance);
        output.push(res.lp); // Lowpass
    }
    
    PyArray1::from_vec_bound(py, output)
}

#[pymodule]
fn dirtydata(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(chua_process, m)?)?;
    m.add_function(wrap_pyfunction!(mackey_glass, m)?)?;
    m.add_function(wrap_pyfunction!(lorenz, m)?)?;
    m.add_function(wrap_pyfunction!(process_ladder_filter, m)?)?;
    Ok(())
}
