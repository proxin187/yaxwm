use yaxi::proto::Event;

use proto::Sequence;

pub enum EventKind {
    XEvent(Event),
    Config(Sequence),
}
