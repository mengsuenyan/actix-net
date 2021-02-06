use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::server::Server;

/// Different types of process signals
#[allow(dead_code)]
#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum Signal {
    /// SIGHUP
    Hup,
    /// SIGINT
    Int,
    /// SIGTERM
    Term,
    /// SIGQUIT
    Quit,
}

pub(crate) struct Signals {
    #[cfg(not(unix))]
    signals: futures_core::future::LocalBoxFuture<'static, std::io::Result<()>>,
    #[cfg(unix)]
    signals: Vec<(Signal, actix_rt::signal::unix::Signal)>,
}

impl Signals {
    pub(crate) fn new() -> Self {
        #[cfg(not(unix))]
        {
            Signals {
                signals: Box::pin(actix_rt::signal::ctrl_c()),
            }
        }
        #[cfg(unix)]
        {
            use actix_rt::signal::unix;

            let sig_map = [
                (unix::SignalKind::interrupt(), Signal::Int),
                (unix::SignalKind::hangup(), Signal::Hup),
                (unix::SignalKind::terminate(), Signal::Term),
                (unix::SignalKind::quit(), Signal::Quit),
            ];

            let signals = sig_map
                .iter()
                .filter_map(|(kind, sig)| {
                    unix::signal(*kind)
                        .map(|tokio_sig| (*sig, tokio_sig))
                        .map_err(|e| {
                            log::error!(
                                "Can not initialize stream handler for {:?} err: {}",
                                sig,
                                e
                            )
                        })
                        .ok()
                })
                .collect::<Vec<_>>();

            Signals { signals }
        }
    }
}

impl Future for Signals {
    type Output = Signal;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[cfg(not(unix))]
        {
            self.signals.as_mut().poll(cx).map(|_| Signal::Int)
        }

        #[cfg(unix)]
        {
            for (sig, fut) in self.signals.iter_mut() {
                if Pin::new(fut).poll_recv(cx).is_ready() {
                    return Poll::Ready(*sig);
                }
            }
            Poll::Pending
        }
    }
}
