pub fn calc_median(data: &[f32]) -> Option<f32> {
    calc_nth_percentile(data, 0.5)
}

pub fn calc_25th_percentile(data: &[f32]) -> Option<f32> {
    calc_nth_percentile(data, 0.25)
}

pub fn calc_75th_percentile(data: &[f32]) -> Option<f32> {
    calc_nth_percentile(data, 0.75)
}

pub fn calc_nth_percentile(data: &[f32], perc: f32) -> Option<f32> {
    if data.is_empty() {
        return None;
    }
    let sorted: Vec<f32> = if data.is_sorted() {
        data.to_vec()
    } else {
        let mut d2 = data.to_vec();
        d2.sort_unstable_by(f32::total_cmp);
        d2
    };
    let rank = perc * (sorted.len() - 1) as f32;
    let lower_idx = rank.floor() as usize;
    let upper_idx = rank.ceil() as usize;
    if lower_idx == upper_idx {
        sorted.get(lower_idx).copied()
    } else {
        Some((*sorted.get(lower_idx)? + *sorted.get(upper_idx)?) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_median_uneven() {
        let data: [f32; 5] = [0.1, 1.2, 0.2, 0.03, 0.04];
        let m = calc_median(&data);
        assert_eq!(m, Some(0.1 as f32));
    }

    #[test]
    fn test_calc_median_even() {
        let data: [f32; 4] = [1.2, 0.2, 0.03, 0.04];
        let m = calc_median(&data);
        assert_eq!(m, Some((0.2 + 0.04) / 2 as f32));
    }

    #[test]
    fn test_calc_25th_percentile_uneven() {
        let data: [f32; 5] = [0.1, 1.2, 0.2, 0.03, 0.04];
        let p = calc_25th_percentile(&data);
        assert_eq!(p, Some(0.04));
    }

    #[test]
    fn test_calc_25th_percentile_even() {
        let data: [f32; 4] = [1.2, 0.2, 0.03, 0.04];
        let p = calc_25th_percentile(&data);
        assert_eq!(p, Some((0.03 + 0.04) / 2 as f32));
    }

    #[test]
    fn test_calc_75th_percentile_uneven() {
        let data: [f32; 5] = [0.1, 1.2, 0.2, 0.03, 0.04];
        let p = calc_75th_percentile(&data);
        assert_eq!(p, Some(0.2));
    }

    #[test]
    fn test_calc_75th_percentile_even() {
        let data: [f32; 4] = [1.2, 0.2, 0.03, 0.04];
        let p = calc_75th_percentile(&data);
        assert_eq!(p, Some((0.2 + 1.2) / 2 as f32));
    }
}
