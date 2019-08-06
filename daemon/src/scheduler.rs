//! Deal with asynchronous scheduled processes, either triggered by an
//! event that occurred, or planned (e.g. re-publishing).

use std::sync::Arc;
use std::time::Duration;

use clokwerk::{self, ScheduleHandle, TimeUnits};

use krill_commons::api::admin::Handle;
use krill_commons::api::publication::PublishDelta;
use krill_commons::util::softsigner::OpenSslSigner;
use krill_pubd::PubServer;

use crate::ca::CaServer;
use crate::mq::{EventQueueListener, QueueEvent};

pub struct Scheduler {
    #[allow(dead_code)] // just need to keep this in scope
    event_sh: ScheduleHandle,

    #[allow(dead_code)] // just need to keep this in scope
    republish_sh: ScheduleHandle,

    #[allow(dead_code)] // just need to keep this in scope
    ca_refresh_sh: ScheduleHandle,
}

impl Scheduler {
    pub fn build(
        event_queue: Arc<EventQueueListener>,
        caserver: Arc<CaServer<OpenSslSigner>>,
        pubserver: Arc<PubServer>,
        ca_refresh_rate: u32,
    ) -> Self {
        let event_sh = make_event_sh(event_queue, caserver.clone(), pubserver);
        let republish_sh = make_republish_sh(caserver.clone());
        let ca_refresh_sh = make_ca_refresh_sh(caserver, ca_refresh_rate);

        Scheduler {
            event_sh,
            republish_sh,
            ca_refresh_sh,
        }
    }
}

fn make_event_sh(
    event_queue: Arc<EventQueueListener>,
    caserver: Arc<CaServer<OpenSslSigner>>,
    pubserver: Arc<PubServer>,
) -> ScheduleHandle {
    let mut scheduler = clokwerk::Scheduler::new();
    scheduler.every(1.seconds()).run(move || {
        while let Some(evt) = event_queue.pop() {
            match evt {
                QueueEvent::Delta(handle, delta) => {
                    publish(&handle, delta, &pubserver);
                }
                QueueEvent::ParentAdded(handle, parent, contact) => {
                    if let Err(e) = caserver.get_updates_from_parent(&handle, &parent, &contact) {
                        error!("Getting updates for {}, error: {}", &handle, e);
                    } else {
                        info!("Parent added, updated certificates for CA {}", &handle);
                    }
                }
            }
        }
    });
    scheduler.watch_thread(Duration::from_millis(100))
}

fn make_republish_sh(caserver: Arc<CaServer<OpenSslSigner>>) -> ScheduleHandle {
    let mut scheduler = clokwerk::Scheduler::new();
    scheduler.every(1.hours()).run(move || {
        // TODO: one by one and keep the result per ca
        if let Err(e) = caserver.republish_all() {
            error!("Publishing failed: {}", e);
        }
    });
    scheduler.watch_thread(Duration::from_millis(100))
}

fn publish(handle: &Handle, delta: PublishDelta, pubserver: &PubServer) {
    debug!("Triggered publishing for CA: {}", handle);
    match pubserver.publish(handle, delta) {
        Ok(()) => debug!("Published for CA: {}", handle),
        Err(e) => error!("Failed to publish for CA: {}, error: {}", handle, e),
    }
}

fn make_ca_refresh_sh(caserver: Arc<CaServer<OpenSslSigner>>, refresh_rate: u32) -> ScheduleHandle {
    let mut scheduler = clokwerk::Scheduler::new();
    scheduler.every(refresh_rate.seconds()).run(move || {
        if let Err(e) = caserver.get_updates_for_all_cas() {
            error!("Failed to refresh CA certificates: {}", e);
        }
    });
    scheduler.watch_thread(Duration::from_millis(100))
}
