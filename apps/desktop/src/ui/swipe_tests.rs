use super::*;
use std::time::Duration;
use TouchPhase::*;

const W: f32 = 1000.;

fn at(t0: Instant, ms: u64) -> Instant {
    t0 + Duration::from_millis(ms)
}

/// Fingers down, `steps` moves of (dx, dy) 16ms apart, then lift.
fn gesture(s: &mut Swipe, dx: f32, dy: f32, steps: u64, can_back: bool, can_fwd: bool) -> Outcome {
    let t0 = Instant::now();
    s.wheel(Started, true, 0., 0., t0, W, can_back, can_fwd);
    for i in 1..=steps {
        s.wheel(Moved, true, dx, dy, at(t0, i * 16), W, can_back, can_fwd);
    }
    s.wheel(Ended, true, 0., 0., at(t0, steps * 16 + 200), W, can_back, can_fwd)
}

#[test]
fn starts_only_on_horizontal_trackpad_gestures() {
    let mut s = Swipe::default();
    // Mouse wheel: never.
    assert_eq!(s.wheel(Moved, false, 40., 0., Instant::now(), W, true, true), Outcome::Ignored);
    // Vertical gesture that later leans sideways: locked to y, never tracks.
    let t0 = Instant::now();
    s.wheel(Started, true, 0., 0., t0, W, true, true);
    assert_eq!(s.wheel(Moved, true, 1., 8., at(t0, 16), W, true, true), Outcome::Ignored);
    assert_eq!(s.wheel(Moved, true, 30., 2., at(t0, 32), W, true, true), Outcome::Ignored);
    assert_eq!(s.wheel(Ended, true, 0., 0., at(t0, 48), W, true, true), Outcome::Ignored);
    // Momentum (no fingers) never starts one.
    assert_eq!(s.wheel(Moved, true, 50., 0., at(t0, 64), W, true, true), Outcome::Ignored);
    // A sideways gesture tracks.
    s.wheel(Started, true, 0., 0., t0, W, true, true);
    assert_eq!(s.wheel(Moved, true, 10., 1., at(t0, 80), W, true, true), Outcome::Moved);
    assert!(s.active());
}

#[test]
fn completion_threshold_and_velocity() {
    // Drag to 40% then rest 200ms: completes back; to 20% and rest: cancels.
    assert_eq!(gesture(&mut Swipe::default(), 25., 0., 16, true, false), Outcome::Released(Some(Dir::Back)));
    assert_eq!(gesture(&mut Swipe::default(), 25., 0., 8, true, false), Outcome::Released(None));
    assert_eq!(release(200., 0., W, true, false), None);
    assert_eq!(release(400., 0., W, true, false), Some(Dir::Back));
    // A fast flick completes from a short distance, not against its direction.
    assert_eq!(release(80., 1500., W, true, false), Some(Dir::Back));
    assert_eq!(release(80., -1500., W, true, false), None);
    assert_eq!(release(20., 1500., W, true, false), None);
    // Forward only with forward history.
    assert_eq!(release(-500., 0., W, true, false), None);
    assert_eq!(release(-500., 0., W, true, true), Some(Dir::Forward));
}

#[test]
fn follows_fingers_with_resistance_and_cancels() {
    let mut s = Swipe::default();
    let t0 = Instant::now();
    s.wheel(Started, true, 0., 0., t0, W, true, false);
    s.wheel(Moved, true, 100., 0., at(t0, 16), W, true, false);
    assert_eq!(s.frame(at(t0, 16), W, true, false).0, 100.);
    // Toward forward with no forward history: rubber band, under 1:1.
    s.wheel(Moved, true, -300., 0., at(t0, 32), W, true, false);
    let (x, _) = s.frame(at(t0, 32), W, true, false);
    assert!(x < 0. && x > -200.);
    // Released short of the threshold: springs back to rest and does not navigate.
    s.wheel(Moved, true, 250., 0., at(t0, 48), W, true, false);
    assert_eq!(s.wheel(Ended, true, 0., 0., at(t0, 400), W, true, false), Outcome::Released(None));
    let mut done = None;
    for i in 1..120 {
        let (x, dir) = s.frame(at(t0, 400 + i * 16), W, true, false);
        assert!(dir.is_none());
        if !s.active() {
            done = Some(x);
            break;
        }
    }
    assert_eq!(done, Some(0.));
}

#[test]
fn completing_settle_reports_the_direction_once() {
    let mut s = Swipe::default();
    assert_eq!(gesture(&mut s, 40., 0., 12, true, false), Outcome::Released(Some(Dir::Back)));
    let t0 = Instant::now();
    let mut got = vec![];
    for i in 1..200 {
        if let (_, Some(d)) = s.frame(at(t0, i * 16), W, true, false) {
            got.push(d);
        }
    }
    assert_eq!(got, vec![Dir::Back]);
    assert!(!s.active());
}
