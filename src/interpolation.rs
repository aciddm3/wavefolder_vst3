#[derive(Debug, Clone)]
pub struct SincResampler {
    table: Vec<Vec<f32>>,
    // TODO impl cycle buffer
    buffer: Vec<f32>,
    oversampling: usize,
    kernel_diameter: usize,
}

impl SincResampler {
    pub fn new(oversampling: usize, kernel_diameter: usize) -> Self {
        let mut table = vec![vec![0.0; kernel_diameter]; oversampling];

        let a = (kernel_diameter / 2) as f32;
        for phase in 0..oversampling {
            let offset = phase as f32 / oversampling as f32;

            for index in 0..kernel_diameter {
                let x = (index as f32 - a + 1.0) - offset;
                table[phase][index] = Self::lanczos_kernel(x, a);
            }

            let sum: f32 = table[phase].iter().sum();
            for coeff in table[phase].iter_mut() {
                *coeff /= sum;
            }
        }

        Self {
            table,
            buffer: vec![0.0; kernel_diameter],
            oversampling,
            kernel_diameter,
        }
    }

    fn lanczos_kernel(x: f32, a: f32) -> f32 {
        if x == 0.0 {
            return 1.0;
        }
        if x.abs() >= a {
            return 0.0;
        }

        let pi_x = std::f32::consts::PI * x;
        // sinc(x) * sinc(x/a)
        (pi_x.sin() / pi_x) * ((pi_x / a).sin() / (pi_x / a))
    }

    pub fn push_sample(&mut self, sample: f32) {
        self.buffer.rotate_right(1);
        self.buffer[0] = sample;
    }

    pub fn get_phase(&self, phase: usize) -> f32 {
        self.table[phase]
            .iter()
            .zip(self.buffer.iter())
            .map(|(coeff, sample)| coeff * sample)
            .sum()
    }
}

#[derive(Debug, Clone)]
pub struct SincDecimator {
    coeffs: Vec<f32>,
    buffer: Vec<f32>,
    kernel_diameter: usize,
}

impl SincDecimator {
    pub fn new(oversampling: usize, kernel_diameter: usize) -> Self {
        let mut coeffs = vec![0.0; kernel_diameter];
        let a = (kernel_diameter / 2) as f32;
        let cutoff = 1.0 / oversampling as f32;

        for i in 0..kernel_diameter {
            let x = i as f32 - a + 1.0;
            coeffs[i] = Self::sinc(x * cutoff) * Self::window(x, a);
        }

        let sum: f32 = coeffs.iter().sum();
        for c in coeffs.iter_mut() {
            *c /= sum;
        }

        Self {
            coeffs,
            buffer: vec![0.0; kernel_diameter],
            kernel_diameter,
        }
    }

    pub fn process_oversampled(&mut self, sample: f32) {
        self.buffer.rotate_right(1);
        self.buffer[0] = sample;
    }

    pub fn get_output(&self) -> f32 {
        self.coeffs
            .iter()
            .zip(&self.buffer)
            .map(|(c, s)| c * s)
            .sum()
    }

    fn sinc(x: f32) -> f32 {
        if x.abs() < 1e-9 {
            return 1.0;
        }
        let pi_x = std::f32::consts::PI * x;
        pi_x.sin() / pi_x
    }

    fn window(x: f32, a: f32) -> f32 {
        // a — это радиус ядра (kernel_size / 2)
        if x.abs() >= a {
            return 0.0;
        }
        Self::sinc(x / a)
    }
}
