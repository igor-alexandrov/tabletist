//! A notification on the Linux desktop, over D-Bus
//! (`org.freedesktop.Notifications`): what mako shows on Omarchy. One
//! action, Review, which the app answers by showing the Agents tab.

use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

use zbus::export::futures_core::Stream;

use super::AgentsEvent;
use crate::backend::Outbox;

/// How long a notification's Review is waited for.
const WAIT: Duration = Duration::from_secs(60 * 60);

/// The action's key the server sends back when Review is clicked. A
/// server that shows no buttons sends it for a click on the notification.
const REVIEW: &str = "default";

/// Shows `summary` and `body`, and says when Review is clicked, for an
/// hour at most. Nothing where no notification server answers: the
/// sidebar's entry says who is here all the same.
pub(super) async fn notify(summary: String, body: String, outbox: Outbox) {
    if let Err(error) = shown(&summary, &body, &outbox).await {
        // Not its words: they name the agent.
        log::warn!("a notification could not be shown: {error}");
    }
}

async fn shown(summary: &str, body: &str, outbox: &Outbox) -> zbus::Result<()> {
    let connection = zbus::Connection::session().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;
    // Before it is shown: a click that comes at once is not missed.
    let mut signals = zbus::MessageStream::for_match_rule(
        zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.Notifications")?
            .build(),
        &connection,
        None,
    )
    .await?;
    let hints: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
    let actions = [REVIEW, "Review"];
    let id: u32 = proxy
        .call(
            "Notify",
            &(
                "Tabletist",
                0u32,
                "",
                summary,
                body,
                &actions[..],
                hints,
                -1i32,
            ),
        )
        .await?;
    let answered = async {
        while let Some(message) = next(&mut signals).await {
            let message = message?;
            let header = message.header();
            match header.member().map(|member| member.as_str()) {
                Some("ActionInvoked") => {
                    let (of, action): (u32, String) = message.body().deserialize()?;
                    if of == id {
                        if action == REVIEW {
                            outbox.emit(crate::backend::Event::Agents(AgentsEvent::Reviewed));
                        }
                        return Ok(());
                    }
                }
                Some("NotificationClosed") => {
                    let (of, _why): (u32, u32) = message.body().deserialize()?;
                    if of == id {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    };
    tokio::time::timeout(WAIT, answered).await.unwrap_or(Ok(()))
}

/// The stream's next item.
async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    std::future::poll_fn(|context| Pin::new(&mut *stream).poll_next(context)).await
}
