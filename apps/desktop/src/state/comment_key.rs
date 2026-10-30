//! Per-comment transient UI (hover toolbar, More menu, reaction picker, context menu) is keyed by
//! the surface that renders the comment plus its id: one comment can show in the diff and in the
//! Conversation panel at once, and only the surface that opened a layer renders it.

/// Where a comment is rendered.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Surface {
    /// Conversation (Activity) list with the pinned composer.
    Activity,
    /// Inline thread on the diff.
    Code,
}

/// One comment on one surface.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CommentKey {
    pub surface: Surface,
    pub id: String,
}

impl CommentKey {
    pub fn new(surface: Surface, id: impl Into<String>) -> Self {
        Self {
            surface,
            id: id.into(),
        }
    }

    /// Stable string for element ids and string-keyed layers (reaction picker).
    pub fn tag(&self) -> String {
        let s = match self.surface {
            Surface::Activity => "activity",
            Surface::Code => "code",
        };
        format!("{s}:{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_comment_on_two_surfaces_has_distinct_keys() {
        let a = CommentKey::new(Surface::Activity, "42");
        let c = CommentKey::new(Surface::Code, "42");
        assert_ne!(a, c);
        assert_ne!(a.tag(), c.tag());
        assert_eq!(a, CommentKey::new(Surface::Activity, "42"));
    }
}
