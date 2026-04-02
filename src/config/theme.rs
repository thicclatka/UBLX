//! Theme name normalization and fuzzy correction when loading overlay TOML.

/// Normalize for comparison: alphanumeric only, lowercased.
pub fn normalize_theme_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Damerau-Levenshtein (optimal string alignment): insert/delete/replace + adjacent transposition.
pub fn theme_distance(a: &str, b: &str) -> usize {
    if a.is_empty() {
        return b.chars().count();
    }
    if b.is_empty() {
        return a.chars().count();
    }
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev_prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0usize; b_chars.len() + 1];
    for (i, ac) in a_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bc) in b_chars.iter().enumerate() {
            let cost = usize::from(ac != bc);
            let mut cell = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            if i > 0 && j > 0 && *ac == b_chars[j - 1] && a_chars[i - 1] == *bc {
                cell = cell.min(prev_prev[j - 1] + 1);
            }
            curr[j + 1] = cell;
        }
        prev_prev.clone_from(&prev);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_chars.len()]
}

pub fn auto_correct_theme_name<'a>(
    input: &'a str,
    valid_theme_names: &'a [&'a str],
) -> Option<&'a str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if valid_theme_names.contains(&trimmed) {
        return Some(trimmed);
    }
    let norm_input = normalize_theme_key(trimmed);
    if norm_input.is_empty() {
        return None;
    }
    for &name in valid_theme_names {
        if normalize_theme_key(name) == norm_input {
            return Some(name);
        }
    }
    let mut best: Option<(&str, f32)> = None;
    let mut second_best = 0.0f32;
    for &name in valid_theme_names {
        let cand = normalize_theme_key(name);
        if cand.is_empty() {
            continue;
        }
        let dist = theme_distance(&norm_input, &cand) as f32;
        let max_len = norm_input.chars().count().max(cand.chars().count()) as f32;
        let score = 1.0 - (dist / max_len);
        match best {
            None => best = Some((name, score)),
            Some((_, s)) if score > s => {
                second_best = s;
                best = Some((name, score));
            }
            _ if score > second_best => second_best = score,
            _ => {}
        }
    }
    let (name, score) = best?;
    if score >= 0.80 && (score - second_best) >= 0.08 {
        Some(name)
    } else {
        None
    }
}
