//! The listener: winit's Wayland connection, our own event queue on it, one `wl_data_device` for
//! the seat, and a thread that reads drops of `text/uri-list`.
#![allow(unsafe_code)] // only `adopt` below; see its safety argument.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use wayland_backend::client::{Backend, ObjectId};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::{wl_registry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, event_created_child};

use super::{Drop, URI_LIST, paths_from_uri_list};

/// How long the thread waits for events before it looks at the stop flag again.
const POLL: Duration = Duration::from_millis(100);

pub(crate) struct Inner {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl std::ops::Drop for Inner {
    /// Stops the thread and waits for it, so no Wayland call of ours is made after this returns.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// winit's connection, adopted.
///
/// SAFETY: `display` is the `wl_display` of winit's own connection, from the window's raw display
/// handle (the caller's guarantee, [`super::Listener::start`]). `from_foreign_display` does not
/// take ownership: it never closes the display. What keeps the pointer valid for as long as we use
/// it: everything that touches it (the `Connection`, our event queue, the proxies) lives on the
/// listener thread, and the thread is stopped and joined when the [`super::Listener`] is dropped;
/// the app drops it in `eframe::App::on_exit`, which eframe calls before winit closes its
/// connection (the window and event loop are torn down after `on_exit` returns). The thread never
/// blocks inside libwayland: it reads only after `poll` says the socket is readable, and otherwise
/// wakes every [`POLL`] to see the stop flag.
fn adopt(display: NonNull<std::ffi::c_void>) -> Connection {
    let backend = unsafe { Backend::from_foreign_display(display.as_ptr().cast()) };
    Connection::from_backend(backend)
}

pub(crate) fn start(display: NonNull<std::ffi::c_void>, notify: Box<dyn Fn() + Send>) -> Option<(Inner, Receiver<Drop>)> {
    let conn = adopt(display);
    let mut queue: EventQueue<State> = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let (tx, rx) = std::sync::mpsc::channel();
    let mut state = State { tx, notify, manager: None, seat: None, device: None, offers: HashMap::new(), current: None, at: (0.0, 0.0), conn: conn.clone() };
    // Two round trips: the globals, then the device made from them.
    queue.roundtrip(&mut state).ok()?;
    let (manager, seat) = (state.manager.clone()?, state.seat.clone()?);
    state.device = Some(manager.get_data_device(&seat, &qh, ()));
    queue.roundtrip(&mut state).ok()?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let thread = std::thread::Builder::new()
        .name("wayland-drop".into())
        .spawn(move || {
            while !stop_thread.load(Ordering::SeqCst) {
                if queue.dispatch_pending(&mut state).is_err() || queue.flush().is_err() {
                    break;
                }
                let Some(guard) = queue.prepare_read() else { continue };
                let readable = {
                    let fd = guard.connection_fd();
                    let mut fds = [rustix::event::PollFd::new(&fd, rustix::event::PollFlags::IN)];
                    let timeout = rustix::event::Timespec { tv_sec: 0, tv_nsec: POLL.as_nanos() as i64 };
                    matches!(rustix::event::poll(&mut fds, Some(&timeout)), Ok(n) if n > 0)
                };
                if readable {
                    if guard.read().is_err() {
                        break;
                    }
                } else {
                    drop(guard); // cancels the read
                }
            }
        })
        .ok()?;
    Some((Inner { stop, thread: Some(thread) }, rx))
}

struct State {
    tx: Sender<Drop>,
    notify: Box<dyn Fn() + Send>,
    manager: Option<WlDataDeviceManager>,
    seat: Option<WlSeat>,
    device: Option<WlDataDevice>,
    /// The MIME types each live offer said it has.
    offers: HashMap<ObjectId, Vec<String>>,
    /// The offer of the drag over our surface now.
    current: Option<WlDataOffer>,
    /// The pointer's last place over our surface during the drag (logical points).
    at: (f64, f64),
    conn: Connection,
}

impl State {
    fn send(&self, d: Drop) {
        if self.tx.send(d).is_ok() {
            (self.notify)();
        }
    }

    fn forget(&mut self, offer: &WlDataOffer) {
        self.offers.remove(&offer.id());
        offer.destroy();
    }

    /// The dropped offer's URI list, read from the source through a pipe.
    fn read_uri_list(&self, offer: &WlDataOffer) -> Option<String> {
        let (mut reader, writer) = std::io::pipe().ok()?;
        offer.receive(URI_LIST.to_owned(), std::os::fd::AsFd::as_fd(&writer));
        drop(writer); // the source writes and closes its copy
        self.conn.flush().ok()?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).ok()?;
        String::from_utf8(bytes).ok()
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "wl_data_device_manager" if state.manager.is_none() => state.manager = Some(registry.bind(name, version.min(3), qh, ())),
                "wl_seat" if state.seat.is_none() => state.seat = Some(registry.bind(name, version.min(1), qh, ())),
                _ => {}
            }
        }
    }
}

impl Dispatch<WlDataDeviceManager, ()> for State {
    fn event(_: &mut Self, _: &WlDataDeviceManager, _: <WlDataDeviceManager as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlSeat, ()> for State {
    fn event(_: &mut Self, _: &WlSeat, _: <WlSeat as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlDataOffer, ()> for State {
    fn event(state: &mut Self, offer: &WlDataOffer, event: wl_data_offer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_data_offer::Event::Offer { mime_type } = event {
            state.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<WlDataDevice, ()> for State {
    fn event(state: &mut Self, _: &WlDataDevice, event: wl_data_device::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            wl_data_device::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            wl_data_device::Event::Enter { serial, x, y, id, .. } => {
                state.at = (x, y);
                if let Some(offer) = id {
                    let files = state.offers.get(&offer.id()).is_some_and(|m| m.iter().any(|t| t == URI_LIST));
                    offer.accept(serial, files.then(|| URI_LIST.to_owned()));
                    if offer.version() >= 3 {
                        let action = if files { DndAction::Copy } else { DndAction::empty() };
                        offer.set_actions(action, action);
                    }
                    state.current = Some(offer);
                }
            }
            wl_data_device::Event::Motion { x, y, .. } => state.at = (x, y),
            wl_data_device::Event::Leave => {
                if let Some(offer) = state.current.take() {
                    state.forget(&offer);
                }
            }
            wl_data_device::Event::Drop => {
                let Some(offer) = state.current.take() else { return };
                let files = state.offers.get(&offer.id()).is_some_and(|m| m.iter().any(|t| t == URI_LIST));
                let paths: Vec<PathBuf> = if files { state.read_uri_list(&offer).map(|l| paths_from_uri_list(&l)).unwrap_or_default() } else { Vec::new() };
                if offer.version() >= 3 && files {
                    offer.finish();
                }
                state.forget(&offer);
                state.send(if paths.is_empty() { Drop::NotFiles } else { Drop::Files { paths, at: state.at } });
            }
            // The clipboard is egui's (smithay-clipboard, on its own data device).
            wl_data_device::Event::Selection { id: Some(offer) } => state.forget(&offer),
            _ => {}
        }
    }

    event_created_child!(State, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}
