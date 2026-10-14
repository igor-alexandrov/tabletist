//! Stopping a call, and remembering why. A call can be told to end by more
//! than one, and it gives one reason: the person's stop if there was one,
//! else the first.

use std::sync::{Arc, OnceLock};

use tokio_util::sync::CancellationToken;

/// Why a call was stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Why {
    /// The client cancelled the call, or is gone.
    Client,
    /// The person stopped the session or the server.
    Stopped,
}

/// A token and the reason it was cancelled for. A child is cancelled with
/// its parent: the server's stop is every session's, and a session's is
/// every call's.
#[derive(Debug, Clone)]
pub(crate) struct Stop {
    token: CancellationToken,
    reasons: Arc<Reasons>,
}

/// A stop's own reason, and its parent's.
#[derive(Debug, Default)]
struct Reasons {
    own: OnceLock<Why>,
    parent: Option<Arc<Reasons>>,
}

impl Stop {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            reasons: Arc::default(),
        }
    }

    /// A stop that ends when this one does, and can be ended alone.
    pub fn child(&self) -> Self {
        Self {
            token: self.token.child_token(),
            reasons: Arc::new(Reasons {
                own: OnceLock::new(),
                parent: Some(self.reasons.clone()),
            }),
        }
    }

    /// Ends whatever holds this stop or a child of it. Of the reasons one
    /// stop is given, the first holds.
    pub fn end(&self, why: Why) {
        let _ = self.reasons.own.set(why);
        self.token.cancel();
    }

    /// Why this stop was ended, or a stop above it. The person's stop wins
    /// wherever it stands: a call that was stopped says so, though
    /// something else ended it as well. Otherwise the nearest reason.
    pub fn why(&self) -> Option<Why> {
        let mut nearest = None;
        let mut at = Some(&self.reasons);
        while let Some(reasons) = at {
            match reasons.own.get() {
                Some(Why::Stopped) => return Some(Why::Stopped),
                Some(why) => nearest = nearest.or(Some(*why)),
                None => {}
            }
            at = reasons.parent.as_ref();
        }
        nearest
    }

    /// For a gateway, which ends what a session is doing when this is
    /// cancelled.
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub async fn ended(&self) {
        self.token.cancelled().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ended(stop: &Stop) -> bool {
        stop.token.is_cancelled()
    }

    #[test]
    fn the_first_reason_holds() {
        let stop = Stop::new();
        assert_eq!(stop.why(), None);
        stop.end(Why::Client);
        stop.end(Why::Stopped);
        assert_eq!(stop.why(), Some(Why::Client));
        assert!(ended(&stop));
    }

    #[test]
    fn a_child_ends_with_its_parent_and_gives_its_reason() {
        // The server's stop, a session's, a call's.
        let server = Stop::new();
        let session = server.child();
        let call = session.child();
        server.end(Why::Stopped);
        assert!(ended(&call));
        assert_eq!(call.why(), Some(Why::Stopped));
        assert_eq!(session.why(), Some(Why::Stopped));
    }

    #[test]
    fn a_child_ended_alone_leaves_its_parent() {
        let session = Stop::new();
        let call = session.child();
        call.end(Why::Client);
        assert!(!ended(&session));
        assert_eq!(session.why(), None);
        assert_eq!(call.why(), Some(Why::Client));
        assert!(!ended(&session.child()));
    }

    #[test]
    fn the_persons_stop_wins_over_what_ended_the_call_besides() {
        // Whichever came first, and from however far up.
        let server = Stop::new();
        let session = server.child();
        let call = session.child();
        call.end(Why::Client);
        server.end(Why::Stopped);
        assert_eq!(call.why(), Some(Why::Stopped));

        let session = Stop::new();
        let call = session.child();
        session.end(Why::Stopped);
        call.end(Why::Client);
        assert_eq!(call.why(), Some(Why::Stopped));
    }
}
