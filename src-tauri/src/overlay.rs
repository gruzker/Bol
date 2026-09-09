use tauri::{PhysicalPosition, PhysicalRect, PhysicalSize};

pub fn position(
    area: &PhysicalRect<i32, u32>,
    size: PhysicalSize<u32>,
    scale: f64,
    anchor: &str,
) -> PhysicalPosition<i32> {
    let margin = (12.0 * scale).round() as i32;
    let axis = |origin: i32, extent: u32, item: u32, alignment: i32| {
        let space = (extent as i64 - item as i64).max(0) as i32;
        let inset = margin.min(space / 2);
        origin
            + match alignment {
                -1 => inset,
                1 => space - inset,
                _ => space / 2,
            }
    };
    let horizontal = if anchor.ends_with("left") {
        -1
    } else if anchor.ends_with("right") {
        1
    } else {
        0
    };
    let vertical = if anchor.starts_with("top") {
        -1
    } else if anchor.starts_with("bottom") {
        1
    } else {
        0
    };
    PhysicalPosition::new(
        axis(area.position.x, area.size.width, size.width, horizontal),
        axis(area.position.y, area.size.height, size.height, vertical),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_positions_fit_work_area_at_different_scales() {
        let area = PhysicalRect {
            position: PhysicalPosition::new(-1920, 40),
            size: PhysicalSize::new(1920, 1000),
        };
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let size = PhysicalSize::new((136.0 * scale) as u32, (64.0 * scale) as u32);
            for anchor in [
                "top-left",
                "top-center",
                "top-right",
                "center-left",
                "center",
                "center-right",
                "bottom-left",
                "bottom-center",
                "bottom-right",
            ] {
                let point = position(&area, size, scale, anchor);
                assert!(point.x >= area.position.x && point.y >= area.position.y);
                assert!(point.x + size.width as i32 <= 0);
                assert!(point.y + size.height as i32 <= 1040);
                if anchor.ends_with("center") || anchor == "center" {
                    assert!((point.x + size.width as i32 / 2 + 960).abs() <= 1);
                }
            }
        }
        assert_eq!(
            position(&area, PhysicalSize::new(136, 64), 1.0, "top-left"),
            PhysicalPosition::new(-1908, 52)
        );
        assert_eq!(
            position(&area, PhysicalSize::new(136, 64), 1.0, "bottom-right"),
            PhysicalPosition::new(-148, 964)
        );
    }
}
