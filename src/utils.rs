#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}
#[inline]
pub fn lookup_custom(table: &[f32], x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }

    let len = table.len() as f32;
    if len < 2.0 {
        return 0.0;
    }

    let mut normalized_x = (x % 4.0) / 4.0;
    if normalized_x < 0.0 {
        normalized_x += 1.0;
    }

    let index_f = normalized_x * (len - 1.0);

    let index_low = (index_f.floor() as usize).max(0);
    let index_high = if index_low + 1 < table.len() {
        index_low + 1
    } else {
        0
    };

    let fract = index_f - index_f.floor();

    table[index_low] * (1.0 - fract) + table[index_high] * fract
}

#[inline]
pub fn triangle(mut x: f32) -> f32 {
    x %= 4.0;
    if x > 2.0 {
        x -= 4.0;
    };
    if x > 1.0 {
        2.0 - x
    } else if x < -1.0 {
        -2.0 - x
    } else {
        x
    }
}

#[inline]
pub fn saw(mut x: f32) -> f32 {
    x %= 4.0;
    if x > 2.0 {
        x -= 4.0;
    } else if x < -2.0 {
        x += 4.0;
    }
    x / 2.0
}

#[inline]
pub fn sine(x: f32) -> f32 {
    (x * std::f32::consts::FRAC_PI_2).sin()
}

#[inline]
pub fn meander(mut x: f32) -> f32 {
    x %= 4.0;
    if x > 2.0 {
        x -= 4.0;
    } else if x < -2.0 {
        x += 4.0;
    }
    x.signum()
}

#[inline]
pub fn xfader(a: f32, b: f32, ratio: f32) -> f32 {
    if a.is_nan() || b.is_nan() || ratio.is_nan() {
        return 0.0;
    };
    a * (ratio * std::f32::consts::FRAC_PI_2).cos()
        + b * (ratio * std::f32::consts::FRAC_PI_2).sin()
}
