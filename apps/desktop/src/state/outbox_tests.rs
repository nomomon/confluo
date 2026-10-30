use super::*;

fn me() -> User {
    User::named("me")
}

fn server(id: &str, body: &str) -> Comment {
    Comment {
        id: id.into(),
        author: me(),
        body: body.into(),
        created_at: Utc::now(),
        position: None,
        reactions: vec![],
        suggestions: vec![],
    }
}

fn ids(ts: &[Thread]) -> Vec<String> {
    ts.iter()
        .flat_map(|t| t.comments.iter().map(|c| c.id.clone()))
        .collect()
}

#[test]
fn optimistic_insert_then_success_replaces_it() {
    let mut o = Outbox::default();
    let cid = o.push(Post::Comment, "hi".into(), me(), None);
    let mut ts = vec![];
    o.apply(&mut ts);
    assert_eq!(ids(&ts), vec![format!("{PREFIX}{cid}")]);
    assert_eq!(
        o.status_of(&format!("{PREFIX}{cid}")),
        Some(&Status::Sending)
    );
    o.set(&cid, Status::Sent(Box::new(server("77", "hi"))));
    o.apply(&mut ts);
    assert_eq!(ids(&ts), vec!["77"]);
    assert_eq!(o.status_of("77"), None);
}

#[test]
fn failure_keeps_the_item_for_retry() {
    let mut o = Outbox::default();
    let cid = o.push(Post::Reply("t".into()), "hi".into(), me(), None);
    o.set(&cid, Status::Failed("GitLab: not found (404)".into()));
    let mut ts = vec![Thread::conversation("t", vec![server("1", "first")])];
    o.apply(&mut ts);
    assert_eq!(ts[0].comments.len(), 2);
    assert!(matches!(
        o.status_of(&format!("{PREFIX}{cid}")),
        Some(Status::Failed(_))
    ));
    // Retry flips it back to Sending in place.
    o.set(&cid, Status::Sending);
    assert_eq!(o.get(&cid).unwrap().status, Status::Sending);
}

#[test]
fn refresh_during_pending_keeps_it() {
    let mut o = Outbox::default();
    o.push(Post::Comment, "hi".into(), me(), None);
    // A refetch from before the post landed: server threads lack it.
    let mut fresh = vec![Thread::conversation("a", vec![server("1", "old")])];
    o.reconcile(&fresh, Some("me"));
    o.apply(&mut fresh);
    assert_eq!(fresh.len(), 2);
    // Applying twice does not duplicate.
    o.apply(&mut fresh);
    assert_eq!(fresh.len(), 2);
}

#[test]
fn reconcile_by_body_or_id() {
    let mut o = Outbox::default();
    o.push(Post::Comment, "hi".into(), me(), None);
    let sent = o.push(Post::Comment, "two".into(), me(), None);
    o.set(&sent, Status::Sent(Box::new(server("9", "two"))));
    let ts = vec![
        Thread::conversation("a", vec![server("5", "hi")]),
        Thread::conversation("b", vec![server("9", "two")]),
    ];
    o.reconcile(&ts, Some("me"));
    assert!(o.items.is_empty());
}

#[test]
fn duplicate_guard_blocks_a_second_submit() {
    let mut o = Outbox::default();
    let cid = o.push(Post::Comment, "hi".into(), me(), None);
    assert!(o.sending("hi "));
    o.set(&cid, Status::Failed("x".into()));
    assert!(!o.sending("hi"));
}

#[test]
fn edit_overlays_the_body_until_confirmed() {
    let mut o = Outbox::default();
    o.push(
        Post::Edit("1".into()),
        "new".into(),
        me(),
        Some("old".into()),
    );
    let mut ts = vec![Thread::conversation("a", vec![server("1", "old")])];
    o.apply(&mut ts);
    assert_eq!(ts[0].comments[0].body, "new");
    assert_eq!(o.status_of("1"), Some(&Status::Sending));
    // A stale refetch still has the old body: kept.
    o.reconcile(
        &[Thread::conversation("a", vec![server("1", "old")])],
        Some("me"),
    );
    assert_eq!(o.items.len(), 1);
    o.reconcile(
        &[Thread::conversation("a", vec![server("1", "new")])],
        Some("me"),
    );
    assert!(o.items.is_empty());
}
