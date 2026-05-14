pub fn password_entropy(pass: &str) -> f64 {
    let has_lowercase = pass.chars().any(|c| c.is_lowercase());
    let has_uppercase = pass.chars().any(|c| c.is_uppercase());
    let has_digit = pass.chars().any(|c| c.is_numeric());
    let has_special = pass.chars().any(|c| "!?.+-*/@$^#&%~`|{}[]()<>".contains(c));

    let pool_size: f32 = match (has_lowercase, has_uppercase, has_digit, has_special) {
        (true, false, false, false) => 26.0,
        (false, true, false, false) => 26.0,
        (true, true, false, false) => 52.0,
        (true, true, true, false) => 62.0,
        (true, true, true, true) => 95.0,
        (true, false, true, false) => 36.0,
        (true, false, false, true) => 58.0,
        (false, false, true, false) => 10.0,
        _ => 72.0,
    };

    ((pass.len() as f64) * (pool_size.log2() as f64))
}

pub fn password_strength(pass: &str) -> &'static str {
    match password_entropy(pass) as u32 {
        0..=59 => "very weak",
        60..=79 => "weak",
        80..=99 => "good",
        100..=119 => "strong",
        _ => "very strong",
    }
}

pub fn strong_password(pass: &str) -> bool {
    let entropy = password_entropy(pass);
    let has_repeated = has_repeated_chars(pass);
    let has_seq = has_sequential(pass);

    entropy >= 80.0 && !has_repeated && !has_seq
}

fn has_repeated_chars(pass: &str) -> bool {
    let chars: Vec<char> = pass.chars().collect();
    chars.windows(3).any(|w| w[0] == w[1] && w[1] == w[2])
}

fn has_sequential(pass: &str) -> bool {
    let chars: Vec<char> = pass.chars().collect();
    chars.windows(3).any(|w| {
        let a = w[0] as i32;
        let b = w[1] as i32;
        let c = w[2] as i32;
        (b == a + 1 && c == b + 1) || (b == a - 1 && c == b - 1)
    })
}