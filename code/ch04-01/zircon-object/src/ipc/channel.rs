use {
    super::*,
    crate::error::*,
    crate::object::*,
    alloc::collections::VecDeque,
    alloc::sync::{Arc, Weak},
    alloc::vec::Vec,
    core::convert::TryInto,
    core::sync::atomic::{AtomicU32, Ordering},
    spin::Mutex,
};

pub struct Channel {
    base: KObjectBase,
    peer: Weak<Channel>,
    recv_queue: Mutex<VecDeque<T>>,
    next_txid: AtomicU32,
}

type T = MessagePacket;
type TxID = u32;

impl_kobject!(Channel
    fn peer(&self) -> ZxResult<Arc<dyn KernelObject>> {
        let peer = self.peer.upgrade().ok_or(ZxError::PEER_CLOSED)?;
        Ok(peer)
    }
    fn related_koid(&self) -> KoID {
        self.peer.upgrade().map(|p| p.id()).unwrap_or(0)
    }
);

impl Channel {
    /// Create a channel and return a pair of its endpoints
    #[allow(unsafe_code)]
    pub fn create() -> (Arc<Self>, Arc<Self>) {
        let mut channel0 = Arc::new(Channel {
            base: KObjectBase::default(),
            peer: Weak::default(),
            recv_queue: Default::default(),
            next_txid: AtomicU32::new(0x8000_0000),
        });
        let channel1 = Arc::new(Channel {
            base: KObjectBase::default(),
            peer: Arc::downgrade(&channel0),
            recv_queue: Default::default(),
            next_txid: AtomicU32::new(0x8000_0000),
        });
        // no other reference of `channel0`
        unsafe {
            Arc::get_mut_unchecked(&mut channel0).peer = Arc::downgrade(&channel1);
        }
        (channel0, channel1)
    }

    /// Read a packet from the channel if check is ok, otherwise the msg will keep.
    pub fn read(&self) -> ZxResult<T> {
        let mut recv_queue = self.recv_queue.lock();
        if recv_queue.front().is_some() {
            let msg = recv_queue.pop_front().unwrap();
            return Ok(msg);
        }
        if self.peer_closed() {
            Err(ZxError::PEER_CLOSED)
        } else {
            Err(ZxError::SHOULD_WAIT)
        }
    }

    /// Write a packet to the channel
    pub fn write(&self, msg: T) -> ZxResult {
        let peer = self.peer.upgrade().ok_or(ZxError::PEER_CLOSED)?;
        peer.push_general(msg);
        Ok(())
    }

    /// Push a message to general queue, called from peer.
    fn push_general(&self, msg: T) {
        let mut send_queue = self.recv_queue.lock();
        send_queue.push_back(msg);
    }

    /// Generate a new transaction ID for `call`.
    fn new_txid(&self) -> TxID {
        self.next_txid.fetch_add(1, Ordering::SeqCst)
    }

    /// Is peer channel closed?
    fn peer_closed(&self) -> bool {
        self.peer.strong_count() == 0
    }
}

/// The message transferred in the channel.
/// See [Channel](struct.Channel.html) for details.
#[derive(Default)]
pub struct MessagePacket {
    /// The data carried by the message packet
    pub data: Vec<u8>,
    /// See [Channel](struct.Channel.html) for details.
    pub handles: Vec<Handle>,
}

impl MessagePacket {
    /// Set txid (the first 4 bytes)
    pub fn set_txid(&mut self, txid: TxID) {
        if self.data.len() >= core::mem::size_of::<TxID>() {
            self.data[..4].copy_from_slice(&txid.to_ne_bytes());
        }
    }

    /// Get txid (the first 4 bytes)
    pub fn get_txid(&self) -> TxID {
        if self.data.len() >= core::mem::size_of::<TxID>() {
            TxID::from_ne_bytes(self.data[..4].try_into().unwrap())
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basics() {
        let (end0, end1) = Channel::create();
        assert!(Arc::ptr_eq(
            &end0.peer().unwrap().downcast_arc().unwrap(),
            &end1
        ));
        assert_eq!(end0.related_koid(), end1.id());

        drop(end1);
        assert_eq!(end0.peer().unwrap_err(), ZxError::PEER_CLOSED);
        assert_eq!(end0.related_koid(), 0);
    }

    #[test]
    fn read_write() {
        let (channel0, channel1) = Channel::create();
        // write a message to each other
        channel0
            .write(MessagePacket {
                data: Vec::from("hello 1"),
                handles: Vec::new(),
            })
            .unwrap();
        channel1
            .write(MessagePacket {
                data: Vec::from("hello 0"),
                handles: Vec::new(),
            })
            .unwrap();

        // read message should success
        let recv_msg = channel1.read().unwrap();
        assert_eq!(recv_msg.data.as_slice(), b"hello 1");
        assert!(recv_msg.handles.is_empty());

        let recv_msg = channel0.read().unwrap();
        assert_eq!(recv_msg.data.as_slice(), b"hello 0");
        assert!(recv_msg.handles.is_empty());

        // read more message should fail.
        assert_eq!(channel0.read().err(), Some(ZxError::SHOULD_WAIT));
        assert_eq!(channel1.read().err(), Some(ZxError::SHOULD_WAIT));
    }

    #[test]
    fn peer_closed() {
        let (channel0, channel1) = Channel::create();
        // write a message from peer, then drop it
        channel1.write(MessagePacket::default()).unwrap();
        drop(channel1);
        // read the first message should success.
        channel0.read().unwrap();
        // read more message should fail.
        assert_eq!(channel0.read().err(), Some(ZxError::PEER_CLOSED));
        // write message should fail.
        assert_eq!(
            channel0.write(MessagePacket::default()),
            Err(ZxError::PEER_CLOSED)
        );
    }
}
