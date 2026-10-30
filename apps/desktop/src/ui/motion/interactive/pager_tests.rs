use super::*;
use std::time::Duration;

fn ms(t0: Instant, n: u64) -> Instant {
    t0 + Duration::from_millis(n)
}

/// Two pages 1000 apart (a tab strip: page 1 sits at -1000).
fn tabs() -> Pager {
    Pager::new(vec![0., -1000.], 0)
}

/// Drag `total` in `steps` 16ms moves, rest `rest` ms, release.
fn drag(p: &mut Pager, total: f32, steps: u64, rest: u64) -> usize {
    let t0 = Instant::now();
    p.begin(t0);
    for i in 1..=steps {
        p.drag(total / steps as f32, ms(t0, i * 16));
    }
    p.release(ms(t0, steps * 16 + rest))
}

fn settle(p: &mut Pager) -> usize {
    let t0 = Instant::now();
    for i in 1..400 {
        if let Some(page) = p.tick(ms(t0, i * 8)) {
            return page;
        }
    }
    panic!("never settled");
}

#[test]
fn projection_picks_page_for_flick_and_slow_drag() {
    // Slow drag a third of the way and rest: back to the start.
    let mut p = tabs();
    assert_eq!(drag(&mut p, -300., 20, 200), 0);
    // Slow drag past half: the next page.
    let mut p = tabs();
    assert_eq!(drag(&mut p, -600., 20, 200), 1);
    // A quick short flick projects past half: the next page.
    let mut p = tabs();
    assert_eq!(drag(&mut p, -150., 3, 0), 1);
    // A flick against the drag direction stays.
    let mut p = tabs();
    let t0 = Instant::now();
    p.begin(t0);
    for i in 1..=10 {
        p.drag(-50., ms(t0, i * 16));
    }
    for i in 11..=14 {
        p.drag(60., ms(t0, i * 16));
    }
    assert_eq!(p.release(ms(t0, 14 * 16)), 0);
    // A lower commit share (swipe back: a third) commits a slow 40% drag.
    let mut p = tabs();
    p.commit = 0.33;
    assert_eq!(drag(&mut p, -400., 20, 200), 1);
}

#[test]
fn rubber_band_past_the_allowed_pages() {
    let mut p = tabs();
    let t0 = Instant::now();
    p.begin(t0);
    p.drag(300., ms(t0, 16));
    assert!(p.position() > 0. && p.position() < 200.);
    // Page 1 not allowed: dragging toward it is resisted too.
    p.set_snaps(vec![0., -1000.], 0, 0);
    p.drag(-900., ms(t0, 32));
    assert!(p.position() < 0. && p.position() > -400.);
    assert_eq!(p.release(ms(t0, 300)), 0);
    assert_eq!(settle(&mut p), 0);
    assert_eq!(p.position(), 0.);
}

#[test]
fn interruption_keeps_position_continuous() {
    let mut p = tabs();
    p.animate_to(1, Instant::now());
    let t0 = Instant::now();
    for i in 1..6 {
        p.tick(ms(t0, i * 16));
    }
    let mid = p.position();
    assert!(mid < 0. && mid > -1000.);
    // Grab mid-flight: no jump; dragging moves on from there.
    p.begin(ms(t0, 100));
    assert_eq!(p.position(), mid);
    p.drag(10., ms(t0, 116));
    assert!((p.position() - (mid + 10.)).abs() < 0.01);
    // Grabbing a rubber-banded overshoot is continuous too.
    let mut q = tabs();
    q.begin(t0);
    q.drag(400., ms(t0, 16));
    q.release(ms(t0, 300));
    q.tick(ms(t0, 316));
    let band = q.position();
    q.begin(ms(t0, 320));
    assert!((q.position() - band).abs() < 0.01);
    q.drag(0.0, ms(t0, 336));
    assert!((q.position() - band).abs() < 0.5);
}

#[test]
fn settles_once_and_reduced_motion_snaps() {
    let mut p = tabs();
    p.animate_to(1, Instant::now());
    assert_eq!(settle(&mut p), 1);
    assert_eq!(p.tick(Instant::now()), None);
    assert_eq!((p.position(), p.progress()), (-1000., 1.));
    let mut r = tabs();
    r.reduced = true;
    r.animate_to(1, Instant::now());
    assert!(!r.moving());
    assert_eq!(r.position(), -1000.);
}
