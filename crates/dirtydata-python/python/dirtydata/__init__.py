from .dirtydata import chua_process, mackey_glass, lorenz, process_ladder_filter

__all__ = [
    "chua_process",
    "mackey_glass",
    "lorenz",
    "process_ladder_filter",
]

# High-level wrappers or convenience functions can be added here
def chua_oscillator(n_samples, alpha=15.6, beta=28.0, rate=1.0, sample_rate=44100):
    """Generates chaotic signal from Chua's circuit."""
    import numpy as np
    dummy_input = np.zeros(n_samples, dtype=np.float32)
    return chua_process(dummy_input, alpha, beta, rate, sample_rate)
