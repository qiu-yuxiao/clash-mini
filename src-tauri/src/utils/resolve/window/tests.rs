#![allow(clippy::unwrap_used)]

use super::resize::{
    OuterRectPx, ResizeDirection, clamp_cursor_axis_delta, clamp_wm_sizing_rect, compute_native_resize_outer_rect,
};
use super::{MAX_HEIGHT, MAX_WIDTH, MINIMAL_HEIGHT, MINIMAL_WIDTH};

#[test]
fn east_and_south_resize_keep_origin_anchored() {
    let start = OuterRectPx {
        left: 500,
        top: 300,
        right: 800,
        bottom: 500,
    };
    let east = compute_native_resize_outer_rect(ResizeDirection::E, start, 0, 0, 50, 0, 1.0);
    assert_eq!(east.left, 500);
    assert_eq!(east.width(), 350);
    assert_eq!(east.height(), 200);

    let south = compute_native_resize_outer_rect(ResizeDirection::S, start, 0, 0, 0, 80, 1.0);
    assert_eq!(south.top, 300);
    assert_eq!(south.width(), 300);
    assert_eq!(south.height(), 280);
}

#[test]
fn west_north_and_nw_resize_lock_right_and_bottom_edges_at_150_dpi() {
    let scale = 1.5;
    let start = OuterRectPx {
        left: 103,
        top: 207,
        right: 103 + 450,
        bottom: 207 + 300,
    };
    let nw = compute_native_resize_outer_rect(ResizeDirection::Nw, start, 0, 0, -37, -23, scale);
    assert_eq!(nw.right, start.right);
    assert_eq!(nw.bottom, start.bottom);
    assert_eq!(nw.width(), 487);
    assert_eq!(nw.height(), 323);

    // 超过最小尺寸向内挤压，右边/底边仍零漂移
    let clamped_w = compute_native_resize_outer_rect(ResizeDirection::W, start, 0, 0, 200, 0, scale);
    assert_eq!(clamped_w.right, start.right);
    assert_eq!(clamped_w.width(), (MINIMAL_WIDTH * scale).round() as i32);
}

#[test]
fn non_dragged_axis_passes_through_without_reclamping() {
    let scale = 1.5;
    // 150% 下允许存在 406 物理像素窄边；仅拖上下边时宽度不得被重算顶大为 407
    let start = OuterRectPx {
        left: 100,
        top: 200,
        right: 100 + 406,
        bottom: 200 + 300,
    };
    let north = compute_native_resize_outer_rect(ResizeDirection::N, start, 0, 0, 0, -40, scale);
    assert_eq!(north.width(), 406);
    assert_eq!(north.height(), 340);
    assert_eq!(north.bottom, start.bottom);
}

#[test]
fn anti_recoil_cursor_anchor_sliding_eliminates_boundary_deadzone() {
    let mut start_cursor = 500;
    // 向右拖拽超出最小宽度 271 很多（cursor 滑到 600）
    let clamped_dx = clamp_cursor_axis_delta(300, 271, 640, false, true, 600, &mut start_cursor);
    assert_eq!(clamped_dx, 29);
    assert_eq!(start_cursor, 571);
    // 反向向左刚挪 1px，立即可拉宽至 272px，零死区
    let reverse_dx = clamp_cursor_axis_delta(300, 271, 640, false, true, 599, &mut start_cursor);
    assert_eq!(reverse_dx, 28);
    assert_eq!(300 - reverse_dx, 272);
}

#[test]
fn wm_sizing_clamps_both_width_and_height_anchored_to_opposite_edge() {
    use windows::Win32::UI::WindowsAndMessaging::{WMSZ_BOTTOMRIGHT, WMSZ_TOPLEFT};

    let mut tl = OuterRectPx {
        left: 400,
        top: 400,
        right: 500,
        bottom: 480,
    };
    assert!(clamp_wm_sizing_rect(WMSZ_TOPLEFT, &mut tl, 1.0));
    assert_eq!(tl.right, 500);
    assert_eq!(tl.bottom, 480);
    assert_eq!(tl.width(), MINIMAL_WIDTH as i32);
    assert_eq!(tl.height(), MINIMAL_HEIGHT as i32);

    let mut br = OuterRectPx {
        left: 100,
        top: 100,
        right: 900,
        bottom: 1200,
    };
    assert!(clamp_wm_sizing_rect(WMSZ_BOTTOMRIGHT, &mut br, 1.0));
    assert_eq!(br.left, 100);
    assert_eq!(br.top, 100);
    assert_eq!(br.width(), MAX_WIDTH as i32);
    assert_eq!(br.height(), MAX_HEIGHT as i32);
}
