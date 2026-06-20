use crate::models::VerificationCheck;

pub fn build_verification(checks: &[VerificationCheck]) -> (String, u32) {
    if checks.is_empty() {
        return ("No data".to_string(), 0);
    }

    let mut score = 0u32;
    let mut total_weight = 0u32;
    for c in checks {
        let weight = if c.result == "N/A" { 0 } else { 1 };
        if weight == 0 {
            continue;
        }
        total_weight += 1;
        score += match c.result.as_str() {
            "Normal" => 100,
            "Modified" => 70,
            "Degraded" => 60,
            "Restricted" => 50,
            "Unknown" => 40,
            _ => 30,
        };
    }

    let final_score = if total_weight > 0 {
        score / total_weight
    } else {
        50
    };

    let status = if final_score >= 85 {
        "No issues found"
    } else if final_score >= 60 {
        "Minor issues detected"
    } else {
        "Issues detected"
    };

    (status.to_string(), final_score)
}
