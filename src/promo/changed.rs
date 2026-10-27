//! Where a frame changed: the box round the pixels that differ from the
//! frame before it. The runner writes it with each event, and a reel's
//! cut frames it when nothing it did has a rect of its own (a key with
//! nothing focused, as History's).

/// The box round every pixel of `after` that differs from `before`, in
/// window points at `scale` pixels a point: x0, y0, x1, y1. Both are RGBA,
/// `width` pixels a row. None where they are the same, or not the same
/// size.
pub(super) fn changed(before: &[u8], after: &[u8], width: usize, scale: f32) -> Option<[f32; 4]> {
    let row = width * 4;
    if before.len() != after.len() || row == 0 || !after.len().is_multiple_of(row) {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (y, (a, b)) in before
        .chunks_exact(row)
        .zip(after.chunks_exact(row))
        .enumerate()
    {
        if a == b {
            continue;
        }
        let differs = |x: &usize| a[x * 4..x * 4 + 4] != b[x * 4..x * 4 + 4];
        let (Some(first), Some(last)) = ((0..width).find(differs), (0..width).rev().find(differs))
        else {
            continue;
        };
        x0 = x0.min(first);
        x1 = x1.max(last + 1);
        y0 = y0.min(y);
        y1 = y + 1;
    }
    (x0 != usize::MAX).then(|| [x0, y0, x1, y1].map(|pixels| pixels as f32 / scale))
}

#[cfg(test)]
mod tests {
    use super::changed;

    /// A `width` by `height` RGBA picture, white, with `dark` pixels black.
    fn picture(width: usize, height: usize, dark: &[(usize, usize)]) -> Vec<u8> {
        let mut pixels = vec![255; width * height * 4];
        for &(x, y) in dark {
            let at = (y * width + x) * 4;
            pixels[at..at + 3].fill(0);
        }
        pixels
    }

    #[test]
    fn the_box_holds_every_pixel_that_changed_in_points() {
        let before = picture(8, 6, &[(1, 1)]);
        let after = picture(8, 6, &[(4, 2), (6, 4)]);
        // (1, 1) went light, (4, 2) and (6, 4) dark: pixels 1 to 7 across,
        // 1 to 5 down, at two pixels a point.
        assert_eq!(changed(&before, &after, 8, 2.0), Some([0.5, 0.5, 3.5, 2.5]));
    }

    #[test]
    fn the_same_picture_or_another_size_has_no_box() {
        let one = picture(8, 6, &[(1, 1)]);
        assert_eq!(changed(&one, &one, 8, 2.0), None);
        assert_eq!(changed(&one, &picture(8, 5, &[]), 8, 2.0), None);
        assert_eq!(changed(&[], &[], 8, 2.0), None);
    }
}
